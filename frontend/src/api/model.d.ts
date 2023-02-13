export type FlowDesignation = "invalidation" | "authentication" | "authorization" | "enrollment" | "recovery" | "unenrollment" | "configuration"
export type FieldType = "null" | "boolean" | "string" | "number" | "object" | "array";
export type UserField = "email" | "name" | "uuid";

export type FlowData = {
    flow: FlowInfo,
    pending_user: PendingUser | null,
    response_error: SubmissionError | null
} & FlowComponent

export interface FlowInfo {
    title: string
}

export type FieldErrorKind =
    | {
        kind: 'invalid',
        message: string
    }
    | {
        kind: 'invalid_type',
        expected: FieldType,
        got: FieldType
    }
    | {
        kind: 'missing'
    }

export type FieldError = {
    field: string
} & FieldErrorKind

export type SubmissionError =
    | ({
        type: 'field',
    } & FieldError)
    | {
        type: 'no_pending_user'
    }

export type FlowComponent =
    | {
        component: 'access_denied' | 'error',
        message: string,
    }
    | ({
        component: 'identification',
        user_fields: Array<UserField>,
        password: PasswordComponentData | null,
    } & Sources)
    | ({
        component: 'password',
    } & PasswordComponentData)
    | {
        component: 'redirect',
        to: string
    }

export interface PasswordComponentData {
    recovery_url: string | null
}

export interface Sources {
    sources: Array<Source>,
    show_source_labels: boolean
}

export interface Source {
    name: string,
    icon_url: string
}

export interface PendingUser {
    name: string,
    avatar_url: string | null
}

export interface PartialUser {
    uid: string,
    name: string,
    avatar_url: string | null
}
