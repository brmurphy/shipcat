# Manifest Merging

The final manifest for a service is formed by merging together various sources.

## Sources

If there are multiple manifest sources for a service, they are reduced by merging each source into the previous. The sources are as follows (from highest precedence to lowest):

1. Region configuration (from the current region in `shipcat.conf`)
1. Global configuration (from the global configuration in `shipcat.conf`)
1. Service's region-specific configuration (`services/$service/$region.yml`)
1. Service's environment-specific configuration (`services/$service/$environment.yml`)
1. Service's configuration (`services/$service/shipcat.yml`)

## Rules

_See [`Manifest#merge`](../shipcat_definitions/src/merge.rs) for the full logic of two manifest sources are merged.

For most top-level properties of a manifest, the values in the higher-precedence source will fully replace the lower-precedence source, if set. For example, if `sidecars` is declared in both `shipcat.yml` and `dev-uk.yml` (assuming `dev-uk` is the current region), the value in `dev-uk.yml` will be used.

Map based values (e.g., `env`) will be merged, so that all entries in the higher-precedence source will be added to the lower-precedence source, overriding the values for keys which appear in both.

The following properties may not be overridden, so should appear in the service's `shipcat.yml`:
- `name`
- `kong`
- `regions`
- `metadata`
