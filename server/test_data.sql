insert into flows(slug, title, designation, authentication) values('test-flow', 'Test Flow', 'authentication', 'ignored');
insert into identification_stages(fields) values(array['email', 'name']::userid_fields[]);
insert into stages(slug, kind, timeout, identification_stage) values ('id-stage', 'identification', 30, 1);
insert into flow_entries(flow, stage, ordering) values (1, 1, 30);