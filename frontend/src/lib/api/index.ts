import { get, writable } from "svelte/store";
import { error, type Cookies } from "@sveltejs/kit";
import { dev } from "$app/environment";
import type { User } from "./types";

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
    api: ApiResponse<T> | null,
}

export function checkResponse<T>(res: Promise<ExtendedResponse<T>>): Promise<SuccessApiResponse<T>> {
    return res.then(res => {
        if (!res) {
            throw error(500, { message: "Response is null" })
        }
        if (!res.api) {
            console.log("Start")
            console.log(res.status)
            if (res.status >= 300 && res.status <= 399) {
                console.log("Target: " + res.headers.get('location'));
            }
            res.clone().text().then(res => { console.log(res) });
            console.log("End")
            throw error(500, { message: "Api responded with unexpected content" });
        }
        if (!res.api.success) {
            console.log("Api Error: " + "Status: " + res.status + " Message: "+ res.api.message)
            throw error(500, { message: "Api responded with error Status: " + res.status })
        }
        return res.api as SuccessApiResponse<T>
    })
}

export class Api {
    readonly baseUrl: string;
    readonly svelteFetch: FetchType;
    readonly svelteCookies: Cookies | null;
    readonly tokenStore = writable<string | null>(null);
    private refreshPromise = writable<Promise<false | string> | null>(null);

    makeLoc(path: string): string {
        return this.baseUrl + "/v1" + path;
    }
    makeInternalLoc(path: string): string {
        return this.baseUrl + "/internal" + path;
    }

    constructor(baseUrl: string, fetch: FetchType, cookies: Cookies | null) {
        this.baseUrl = baseUrl;
        this.svelteFetch = fetch;
        this.svelteCookies = cookies;
    }

    private updateInit(init: RequestInit): RequestInit {
        const token = get(this.tokenStore)
        if (!token) {
            console.log("No TOken")
            return init
        }
        const updatedHeaders = new Headers(init?.headers);
        updatedHeaders.append('Authorization', `Bearer ${token}`);
        init.headers = updatedHeaders;
        return init
    }

    makeRequest<T = any>(input: string, init?: RequestInit, internal: boolean = false): Promise<ExtendedResponse<T>> {
        const loc = internal ? this.makeInternalLoc(input) : this.makeLoc(input);
        return this.internalRequest({
            input: loc,
            init,
            retry: true
        })
    }

    get<T = any>(input: string, init?: RequestInit, internal: boolean = false): Promise<ExtendedResponse<T>> {
        return this.makeRequest(input, {method: 'get', ...init}, internal)
    }

    post<T = any>(input: string, init?: RequestInit, internal: boolean = false): Promise<ExtendedResponse<T>> {
        return this.makeRequest(input, {method: 'post', ...init}, internal)
    }
    put<T = any>(input: string, init?: RequestInit, internal: boolean = false): Promise<ExtendedResponse<T>> {
        return this.makeRequest(input, {method: 'put', ...init}, internal)
    }
    delete<T = any>(input: string, init?: RequestInit, internal: boolean = false): Promise<ExtendedResponse<T>> {
        return this.makeRequest(input, {method: 'delete', ...init}, internal)
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
                        secure: !dev
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

        const api: { api: ApiResponse<T> | null } = {
            api: null,
        };
        try {
            api.api = await res.clone().json();
        } catch {
            api.api = null;
        }

        return Object.assign(res.clone(), api);
    }

    private async internalRequest<T>(config: RequestConfig): Promise<ExtendedResponse<T>> {
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

    async me(): Promise<User | null> {
        try {
            const res = await this.get('/users/@me');
            if (res.api && res.api.success) {
                return res.api.response
            } else {
                return null
            }
        } catch (err) {
            return null;
        }
    }
}