import { HoppRESTRequest } from "@hoppscotch/data"
import * as TE from "fp-ts/TaskEither"
import * as E from "fp-ts/lib/Either"

import { GQLError } from "~/helpers/backend/GQLClient"
import {
  CreateShortcodeMutation,
  GetInviteDetailsQuery,
  GetInviteDetailsQueryVariables,
  GetMyTeamsQuery,
  GetUserShortcodesQuery,
  TeamAccessRole,
} from "~/helpers/backend/graphql"

import { useGQLQuery } from "~/composables/graphql"
import { Email } from "~/helpers/backend/types/Email"
import { TeamName } from "~/helpers/backend/types/TeamName"
import {
  AcceptTeamInvitationMutation,
  CreateTeamInvitationMutation,
  CreateTeamMutation,
} from "../helpers/backend/graphql"

export type BackendPlatformDef = {
  // Read actions via GQL queries
  getInviteDetails: <GetInviteDetailsError extends string>(
    inviteID: string
  ) => ReturnType<
    typeof useGQLQuery<
      GetInviteDetailsQuery,
      GetInviteDetailsQueryVariables,
      GetInviteDetailsError
    >
  >

  getUserShortcodes: (
    cursor?: string
  ) => Promise<E.Either<GQLError<"">, GetUserShortcodesQuery>>

  // Sample use case for `matchAllTeams` would be at the cloud platform level where the list of teams across instances is fetched
  // and not limited to a single instance.
  getUserTeams: (
    cursor?: string,
    matchAllTeams?: boolean
  ) => Promise<E.Either<GQLError<"">, GetMyTeamsQuery>>

  // Write actions via GQL mutations
  createTeam: <CreateTeamErrors extends string>(
    name: TeamName
  ) => TE.TaskEither<GQLError<CreateTeamErrors>, CreateTeamMutation>

  createTeamInvitation: <CreateTeamInvitationErrors extends string>(
    inviteeEmail: Email,
    inviteeRole: TeamAccessRole,
    teamID: string
  ) => TE.TaskEither<
    GQLError<CreateTeamInvitationErrors>,
    CreateTeamInvitationMutation
  >

  acceptTeamInvitation: <AcceptTeamInvitationErrors extends string>(
    inviteID: string
  ) => TE.TaskEither<
    GQLError<AcceptTeamInvitationErrors>,
    AcceptTeamInvitationMutation
  >

  createShortcode: (
    request: HoppRESTRequest,
    properties?: string
  ) => TE.TaskEither<GQLError<string>, CreateShortcodeMutation>
}
