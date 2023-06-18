export const UserRoles = ['logs', 'developer', 'admin'];
export type UserRole = typeof UserRoles[number];

export interface User {
    name: string,
    roles: UserRole[],
    require_password_reset: boolean,
}