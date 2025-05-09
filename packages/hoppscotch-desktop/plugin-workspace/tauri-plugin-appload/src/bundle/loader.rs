use std::sync::Arc;

use super::{BundleError, Result, VerifiedBundle};
use crate::{
    api::ApiClient,
    cache::CacheManager,
    storage::{StorageError, StorageManager},
    verification::{BundleVerifier, KeyManager},
};

#[derive(Debug)]
struct BundleInfo {
    name: String,
    content: Vec<u8>,
}

pub struct BundleLoader {
    cache: Arc<CacheManager>,
    storage: Arc<StorageManager>,
}

impl BundleLoader {
    pub fn new(cache: Arc<CacheManager>, storage: Arc<StorageManager>) -> Self {
        Self { cache, storage }
    }

    pub async fn load_bundle(&self, server_url: &str) -> Result<()> {
        tracing::info!(%server_url, "Starting bundle loading process");

        let api_client = self.create_api_client(server_url)?;
        let verifier = self.init_bundle_verifier(&api_client).await?;
        let metadata = self.fetch_metadata(&api_client, server_url).await?;

        let bundle_info = match self.get_bundle_info(server_url, &metadata).await? {
            Some(info) => info,
            None => {
                self.download_and_verify_bundle(server_url, &api_client, &verifier, &metadata)
                    .await?
            }
        };

        self.store_verified_bundle(server_url, bundle_info, &metadata)
            .await?;

        tracing::info!(%server_url, "Bundle loading completed successfully");
        Ok(())
    }

    async fn get_bundle_info(
        &self,
        server_url: &str,
        metadata: &crate::BundleMetadata,
    ) -> Result<Option<BundleInfo>> {
        match self.storage.get_bundle_entry(server_url).await? {
            Some(entry) if entry.version == metadata.version => {
                tracing::info!(%server_url, "Bundle version matches, checking cached version");
                match self.load_existing_bundle(&entry.bundle_name).await {
                    Ok(bundle) => {
                        tracing::info!(%server_url, "Successfully loaded cached bundle");
                        Ok(Some(bundle))
                    }
                    Err(BundleError::Storage(StorageError::BundleNotFound(_))) => {
                        tracing::info!(%server_url, "Cached bundle not found, will download fresh copy");
                        Ok(None)
                    }
                    Err(e) => Err(e),
                }
            }
            Some(entry) => {
                tracing::info!(
                    %server_url,
                    local_version = %entry.version,
                    remote_version = %metadata.version,
                    "Version mismatch, downloading new bundle"
                );
                self.storage
                    .delete_bundle(&entry.bundle_name, server_url)
                    .await?;
                Ok(None)
            }
            None => {
                tracing::info!(%server_url, "No existing bundle found, downloading");
                Ok(None)
            }
        }
    }

    async fn store_verified_bundle(
        &self,
        server_url: &str,
        bundle_info: BundleInfo,
        metadata: &crate::BundleMetadata,
    ) -> Result<()> {
        let verified = VerifiedBundle::new(bundle_info.content, metadata.clone())?;

        tracing::info!(%server_url, "Caching verified bundle");
        Ok(self
            .cache
            .cache_bundle(&bundle_info.name, &verified)
            .await?)
    }

    fn create_api_client(&self, server_url: &str) -> Result<ApiClient> {
        Ok(ApiClient::new(server_url).map_err(|e| {
            tracing::error!(%server_url, %e, "Failed to create API client");
            e
        })?)
    }

    async fn init_bundle_verifier(&self, api_client: &ApiClient) -> Result<BundleVerifier> {
        let key_manager = KeyManager::new(api_client.clone()).await.map_err(|e| {
            tracing::error!(%e, "Failed to initialize key manager");
            e
        })?;

        Ok(BundleVerifier::new(key_manager))
    }

    async fn fetch_metadata(
        &self,
        api_client: &ApiClient,
        server_url: &str,
    ) -> Result<crate::BundleMetadata> {
        Ok(api_client
            .fetch_bundle_metadata("latest")
            .await
            .map_err(|e| {
                tracing::error!(%server_url, %e, "Failed to fetch bundle metadata");
                e
            })?)
    }

    async fn load_existing_bundle(&self, bundle_name: &str) -> Result<BundleInfo> {
        tracing::info!(%bundle_name, "Loading bundle from storage");
        let content = self.storage.load_bundle(bundle_name).await?;
        Ok(BundleInfo {
            name: bundle_name.to_string(),
            content,
        })
    }

    async fn download_and_verify_bundle(
        &self,
        server_url: &str,
        api_client: &ApiClient,
        verifier: &BundleVerifier,
        metadata: &crate::BundleMetadata,
    ) -> Result<BundleInfo> {
        tracing::info!(%server_url, "Downloading bundle");
        let content = api_client.download_bundle("latest").await.map_err(|e| {
            tracing::error!(%server_url, %e, "Failed to download bundle");
            e
        })?;

        tracing::info!(%server_url, "Verifying bundle integrity");
        verifier.verify_bundle(&content, metadata).await?;

        let verified = VerifiedBundle::new(content.clone(), metadata.clone())?;
        let name = self.generate_bundle_name(server_url);

        tracing::info!(%server_url, "Storing verified bundle");
        self.storage
            .store_bundle(&name, server_url, &metadata.version, &verified)
            .await?;

        Ok(BundleInfo { name, content })
    }

    fn generate_bundle_name(&self, server_url: &str) -> String {
        server_url
            // NOTE: This is to fix capitalized URL not being served properly on Windows
            .to_lowercase()
            .split("://")
            .nth(1)
            .unwrap_or("unknown")
            // Extract only the hostname part by splitting at the first '/' and taking the first part
            .split('/')
            .next()
            .unwrap_or("unknown")
            .replace(['/', '.', ':', '-'], "_")
            .trim_end_matches('_')
            .to_string()
    }
}
