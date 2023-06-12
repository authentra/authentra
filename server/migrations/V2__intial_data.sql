insert into users(name,password,roles) values('admin','$argon2id$v=19$m=4096,t=3,p=1$yoJiFcBBUR0Ut64Wfdipsw$07BhsPuU4/xPiOEeTIBNjByheu3z79NUjgUNQau+n1M', array ['admin']::user_roles[]);

insert into application_groups(id,scopes) values ('first-party', array ['profile:read', 'profile:write']::internal_scopes[]);