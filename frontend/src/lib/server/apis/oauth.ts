import type { Api, ExtendedResponse } from "$lib/api";
import { InternalScopeObj, type InternalScope, type InternalScopeName } from "$lib/api/developer";
import { redirect } from "@sveltejs/kit";

export interface EarlyOAuthCheck {
    app_name: string,
    invalid_scopes: string[],
    scopes: InternalScopeName[]
}

export interface OAuthCheck {
    app_name: string,
    invalid_scopes: string[],
    scopes: InternalScope[]
}

type OAuthErrorKindCommon = 'invalid_request' | 'unauthorized_client' | 'invalid_scope';
type OAuthErrorKindAuthorize = 'access_denied' | 'unsupported_response_type' | 'server_error' | 'temporarily_unavailable';
type OAuthErrorKindToken = 'invalid_client' | 'invalid_grant' | 'unsupported_grant_type';
type OAuthErrorKindNotSpec = 'invalid_client' | 'invalid_redirect_uri';
export type OAuthErrorKind = OAuthErrorKindCommon & OAuthErrorKindAuthorize & OAuthErrorKindToken & OAuthErrorKindNotSpec;

export interface OAuthError {
    error: OAuthErrorKind,
    error_description: string | undefined,
    error_uri: string | undefined
}

export type OAuthResponse<T> = ({ success: true } & T) | ({ success: false } & OAuthError) | { success: 'redirect', code: number, location: string, makeRedirect: () => never }

export class OAuthApi {
    private api: Api;

    constructor(api: Api) {
        this.api = api;
    }

    private makeOAuthResponse<T>(response: Promise<ExtendedResponse<T>>): Promise<OAuthResponse<T>> {
        return response.then(response => {
            if (response.api) {
                //@ts-expect-error
                const api = response.api as OAuthResponse;
                if (response.status == 200) {
                    api.success = true;
                } else {
                    api.success = false;
                }
                if (api.success == true) {
                    return Promise.resolve(Object.assign({ success: true }, api.response));
                } else {
                    return Promise.resolve(api)
                }
            } else if (response.status >= 300 && response.status <= 399) {
                const code = response.status;
                const location = response.headers.get('location');
                return Promise.resolve({
                    success: 'redirect',
                    code,
                    location,
                    makeRedirect: () => {
                        //@ts-expect-error
                        throw redirect(code, location)
                    }
                })
            } else {
                response.clone().text().then(text => console.log("Status: " + response.status + " Text: " + text));
                return Promise.reject("Malformed response")
            }
        })
    }

    async check(parameters: URLSearchParams): Promise<OAuthResponse<OAuthCheck>> {
        const res: OAuthResponse<EarlyOAuthCheck> = await this.makeOAuthResponse(this.api.get('/oauth/authorize?' + parameters.toString(), {
            redirect: 'manual'
        }, true));
        if (res.success === true) {
            const test = res.scopes.map(scope => { return { name: scope, description: InternalScopeObj[scope] } });
            //@ts-expect-error
            res.scopes = test;
            //@ts-expect-error
            return res as OAuthResponse<OAuthCheck>
        } else {
            return res
        }
    }
    post(parameters: URLSearchParams): Promise<OAuthResponse<void>> {
        return this.makeOAuthResponse(this.api.post('/oauth/authorize?' + parameters.toString(), {
            redirect: 'manual'
        }, true));
    }
}