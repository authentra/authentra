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
from tenants
where uid = $1