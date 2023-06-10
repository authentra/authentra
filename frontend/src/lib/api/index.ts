import { get, writable } from "svelte/store";
import { UserApi } from "./user";
import type { Cookies } from "@sveltejs/kit";

export type FetchType = (input: RequestInfo | URL, init?: RequestInit | undefined) => Promise<Response>;

export interface RequestConfig {
    input: RequestInfo | URL,
    init?: RequestInit,
    retry?: boolean,
}

export type SuccessApiResponse<T> = { success: true, response: T };

export type FailedApiResponse = { success: false, message: string };

export type ApiResponse<T> = SuccessApiResponse<T> | FailedApiResponse;

export interface ExtendedResponse<T> extends Response {
    api: ApiResponse<T> | null
}

export class Api {
    readonly baseUrl: string;
    readonly svelteFetch: FetchType;
    readonly svelteCookies: Cookies | null;
    readonly tokenStore = writable<string | null>(null);
    private refreshPromise = writable<Promise<false | string> | null>(null);

    readonly user: UserApi

    makeLoc(path: string): string {
        return this.baseUrl + path;
    }

    constructor(baseUrl: string, fetch: FetchType, cookies: Cookies | null) {
        this.baseUrl = baseUrl;
        this.svelteFetch = fetch;
        this.svelteCookies = cookies;

        this.user = new UserApi(this)
    }

    private updateInit(init: RequestInit): RequestInit {
        const token = get(this.tokenStore)
        if (!token) {
            return init
        }
        const updatedHeaders = new Headers(init?.headers);
        updatedHeaders.append('Authorization', `Bearer ${token}`);
        init.headers = updatedHeaders;
        return init
    }

    makeRequest<T = any>(input: string, init?: RequestInit): Promise<ExtendedResponse<T>> {
        const loc = this.makeLoc(input);
        return this.internalRequest({
            input: loc,
            init,
            retry: true
        })
    }

    get<T = any>(input: string, init?: RequestInit): Promise<ExtendedResponse<T>> {
        return this.makeRequest(input, {
            method: 'get',
            ...init
        })
    }

    post<T = any>(input: string, init?: RequestInit): Promise<ExtendedResponse<T>> {
        return this.makeRequest(input, {
            method: 'post',
            ...init
        })
    }
    delete<T = any>(input: string, init?: RequestInit): Promise<ExtendedResponse<T>> {
        return this.makeRequest(input, {
            method: 'delete',
            ...init
        })
    }

    refreshToken(): Promise<false | string> {
        const savedPromise = get(this.refreshPromise);
        if (savedPromise) {
            return savedPromise;
        }
        const promise = (async () => {
            console.log("Refreshing jwt")
            const res = await this._extendResponse<string>(await this.svelteFetch(this.makeLoc('/auth/browser/refresh')));
            if (res.ok && res.api && res.api.success) {
                if (this.svelteCookies) {
                    this.svelteCookies.set('jwt', res.api.response, {
                        httpOnly: true,
                        path: '/',
                        sameSite: 'strict',
                    })
                }
                this.tokenStore.set(res.api.response);
                return res.api.response;
            } else {
                if (this.svelteCookies) {
                    this.svelteCookies.delete('jwt')
                }
                this.tokenStore.set(null);
            }
            console.log(`Refresh failed with code ${res.status} and text '${await res.text()}'`);
            return false
        })();
        this.refreshPromise.set(promise);
        return promise;
    }

    async _extendResponse<T>(res: Response): Promise<ExtendedResponse<T>> {
        const nres = res as ExtendedResponse<T>;
        try {
            nres.api = await res.clone().json();
        }catch {
            nres.api = null;
        }
        return nres
    }

    private async internalRequest<T>(config: RequestConfig): Promise<ExtendedResponse<T>> {
        console.debug('Request to: ' + config.input);
        const finalInit = this.updateInit(config.init ? config.init : {});
        const res = await this._extendResponse<T>(await this.svelteFetch(config.input, finalInit));
  
        
        if (res.ok || (res.status >= 300 && res.status <= 399)) {
            return res
        }
        
        if (res.status == 401 && !res.api?.success) {
            if (res.api?.message == 'JWT: Expired' && config.retry) {
                await this.refreshToken();
                config.retry = false;
                return this.internalRequest(config);
            } else {
                console.log('Received 401, message is not jwt expired. Text: \'' + res.api?.message + '\'')
            }
        }
        return res
    }
}