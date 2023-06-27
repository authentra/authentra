import { jsonBody } from "$lib/utils";
import { checkResponse, type Api } from ".";

export const InternalScopes = ['email', 'profile:read', 'profile:write'] as const;
export type InternalScope = typeof InternalScopes[number];

export const ApplicationKinds = ['web-server', 'spa'];
export type ApplicationKind = typeof ApplicationKinds[number];

export interface Application {
    id: string,
    name: string,
    system_application: boolean,
    application_group: string,
    kind: ApplicationKind,
    client_id: string,
    redirect_uri: string[]
}

export interface ApplicationGroup {
    id: string,
    scopes: InternalScope[]
}


export class ApplicationGroupApi {
    private api: Api;
    constructor(api: Api) {
        this.api = api;
    }

    all(): Promise<ApplicationGroup[]> {
        return checkResponse(this.api.get('/application-groups')).then(res => res.response)
    }

    replace(id: string, scopes: InternalScope[]) {
        return checkResponse(this.api.put('/application-groups/' + id, {
            ...jsonBody({ scopes })
        })).then(res => res.response)
    }
    
    usedBy(id: string): Promise<string[]> {
        return checkResponse(this.api.get(`/application-groups/${id}/usages`)).then(res => res.response)
    }
    delete(id: string): Promise<string[]> {
        return checkResponse(this.api.delete(`/application-groups/${id}`)).then(res => res.response)
    }
    create(id: string, scopes: InternalScope[]): Promise<string[]> {
        return checkResponse(this.api.post(`/application-groups`, { ...jsonBody({ id, scopes }) })).then(res => res.response)
    }
}

export class ApplicationApi {
    private api: Api;
    constructor(api: Api) {
        this.api = api;
    }

    all(): Promise<Application[]> {
        return checkResponse(this.api.get('/applications')).then(res => res.response)
    }

    replace(id: string, name: string, redirect_uri: string[]) {
        return checkResponse(this.api.put('/applications/' + id, {
            ...jsonBody({ name, redirect_uri })
        })).then(res => res.response)
    }
    
    delete(id: string): Promise<void> {
        return checkResponse(this.api.delete(`/applications/${id}`)).then(res => res.response)
    }
    create(name: string, application_group: string, kind: ApplicationKind, redirect_uri: string[]): Promise<string[]> {
        return checkResponse(this.api.post(`/applications`, { ...jsonBody({ name, application_group, kind, redirect_uri}) })).then(res => res.response)
    }
}