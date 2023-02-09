import axios from "axios";
import { querystring } from "svelte-spa-router";
import { get } from 'svelte/store';
import type { FlowData } from "./model";

export const base_url = "http://127.0.0.1:8080/api/v1";

export const axios_instance = axios.create({
    baseURL: base_url,
    withCredentials: true
});

export function get_flow(flow_slug: string, query?: string) {
    return axios_instance.get<FlowData>("/flow/executor/" + flow_slug, {
        params: {
            query: query
        }
    })
}

export function post_flow(flow_slug: string, form: HTMLFormElement, query?: string) {
    let form_data = new FormData(form);
    form_data = new URLSearchParams(form_data);
    
    return axios_instance.post<FlowData>("/flow/executor/"+flow_slug, form_data, {
        params: {
            query: query
        }
    })
}

export function get_user_id() {
    return axios_instance.get("/auth")
}