insert into users(name,password,roles,customer) values('admin','$argon2id$v=19$m=4096,t=3,p=1$yoJiFcBBUR0Ut64Wfdipsw$07BhsPuU4/xPiOEeTIBNjByheu3z79NUjgUNQau+n1M', array ['admin']::user_roles[], false);

insert into application_groups(id,scopes,allow_implicit_consent) values ('first-party', array ['profile:read', 'profile:write']::internal_scopes[], true);
insert into application_groups(id,scopes) values ('third-party', array ['profile:read', 'profile:write']::internal_scopes[]);
insert into developer_allowed_groups(id) values('third-party');