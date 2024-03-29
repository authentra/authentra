/// <reference types="@sveltejs/kit" />
/// <reference types="unplugin-icons/types/svelte" />

// See https://kit.svelte.dev/docs/types#app

import type { Api } from "$lib/api";
import type { AdminApi } from "$lib/api/admin";
import type { User } from "$lib/api/types";
import type { Apis } from "$lib/server/apis";

// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		interface Locals {
			api: Api,
			admin: AdminApi,
			apis: Apis,
			user: User | null,
		}
		// interface PageData {}
		// interface Platform {}
	}
}

export {};
