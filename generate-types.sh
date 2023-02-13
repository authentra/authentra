redocly bundle openapi.yaml -o bundle.yaml
openapi-typescript bundle.yaml --output frontend/src/api/schema.ts --immutable-types