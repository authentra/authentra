import type { ApplicationApi, ApplicationGroupApi } from "$lib/api/developer";

export interface Apis {
    applications: ApplicationApi,
    application_groups: ApplicationGroupApi,
}