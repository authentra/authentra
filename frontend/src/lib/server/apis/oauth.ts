import { checkResponse, type Api, type ExtendedResponse } from "$lib/api";
import { redirect } from "@sveltejs/kit";

export interface OAuthCheck {
    app_name: string,
    invalid_scopes: string[],
    scopes: string[]
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

export type OAuthResponse<T> = ({success: true } & T) | ({ success: false  } & OAuthError) | {success: 'redirect', code: number, location: string, makeRedirect: () => never }

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
                }else {
                    api.success = false;
                }
                if (api.success == true) {
                    return Promise.resolve(Object.assign({success: true}, api.response));
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
                response.clone().text().then(text => console.log("Status: " + response.status +" Text: " + text));
                return Promise.reject("Malformed response")
            }
        })
    }

    check(parameters: URLSearchParams): Promise<OAuthResponse<OAuthCheck>> {
        return this.makeOAuthResponse(this.api.get('/oauth/authorize?'+parameters.toString(), {
            redirect: 'manual'
        }, true));
    }
    post(parameters: URLSearchParams): Promise<OAuthResponse<void>> {
        return this.makeOAuthResponse(this.api.post('/oauth/authorize?'+parameters.toString(), {
            redirect: 'manual'
        }, true));
    }
}