import { checkResponse, type Api } from "$lib/api";

export interface OAuthCheck {
    app_name: string,
    invalid_scopes: string[],
    scopes: string[]
}

export class OAuthApi {
    private api: Api;

    constructor(api: Api) {
        this.api = api;
    }

    check(parameters: URLSearchParams): Promise<OAuthCheck> {
        return checkResponse(this.api.get('/oauth/authorize?'+parameters.toString(), {
            redirect: 'manual'
        }, true)).then(res => res.response)
    }
}