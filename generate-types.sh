redocly bundle openapi.yaml -o bundle.yaml
openapi-typescript bundle.yaml --output frontend-new/src/api/schema.ts --immutable-types