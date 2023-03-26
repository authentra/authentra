INSERT INTO users (name, display_name, password, administrator) VALUES ('admin', 'Admin', '$argon2id$v=19$m=4096,t=3,p=1$SFh24ZI4Bh5ZG4nDQ+Jawg$LDmw0mjZr29cP1kf9T9iluWFzhqOYqWloj1iaM2ebDw', true);

insert into flows(slug, title, designation, authentication)
values ('test-flow', 'Test Flow', 'authentication', 'none');
insert into stages(slug, kind, timeout, identification_fields)
values ('id-stage', 'identification', 30, array ['email', 'name']::userid_field[]);
insert into stages(slug, kind, timeout)
values ('password-stage', 'password', 30);
insert into stages(slug, kind, timeout)
values ('login-stage', 'user_login', 30);
insert into flow_entries(flow, stage, ordering)
values (1, 1, 30);
insert into flow_entries(flow, stage, ordering)
values (1, 2, 60);
insert into flow_entries(flow, stage, ordering)
values (1, 3, 90);
insert into tenants(host, is_default, title, logo, favicon, authentication_flow)
values ('authust-default', true, 'Authust', '/static/logo.png', '/static/favicon.png', 1);
select uid,
       host,
       is_default as "default",
       title,
       logo,
       favicon,
       invalidation_flow,
       authentication_flow,
       authorization_flow,
       enrollment_flow,
       recovery_flow,
       unenrollment_flow,
       configuration_flow
from tenants;