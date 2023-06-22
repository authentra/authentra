import type { ApplicationApi, ApplicationGroupApi } from "$lib/api/developer";
import type { OAuthApi } from "./oauth";
import type { UserApi } from "./user";

export interface Apis {
    applications: ApplicationApi,
    application_groups: ApplicationGroupApi,
    users: UserApi,
    oauth: OAuthApi,
}