# Screenshaver Database Schema History

## Purpose

This document is the permanent historical record of Screenshaver
database schema versions and the compatibility rules that govern changes
to the Screenshaver database.

It records:

-   the meaning of each released database schema version;
-   the Screenshaver application version in which each schema was
    introduced;
-   the classification of durable, factory, derived, transient, and
    provenance data;
-   migration requirements between historical schemas and the current
    schema;
-   compatibility decisions that future database work must preserve.

This document is intended to remain useful even after the implementation
associated with an older schema has become obsolete. Historical schema
definitions, historical readers, and representative historical database
fixtures are compatibility assets and must be retained.

------------------------------------------------------------------------

## Schema Versioning Policy

Screenshaver database schemas use monotonically increasing integer
version numbers:

``` text
1, 2, 3, 4, ...
```

Schema versions are not dates and are not derived from Screenshaver
application version numbers.

Three values have distinct meanings:

-   **Database schema version** identifies the structural and semantic
    database contract.
-   **Screenshaver application version** identifies the application
    release that introduced or used that contract.
-   **Introduction date** records historical chronology.

A database schema-version increment requires a Screenshaver
application-version increment.

A Screenshaver application-version increment does **not** necessarily
require a database schema-version increment.

Once a database schema has been publicly released, its schema definition
and its association with the introducing Screenshaver version are
historical facts and must not be redefined.

Development revisions made before a schema is publicly released do not
consume additional permanent schema numbers. For example, several
revisions to the proposed Schema 2 during development remain revisions
of the Schema 2 candidate until Schema 2 is released.

------------------------------------------------------------------------

## Permanent Compatibility Contract

Beginning with Database Schema Version 1, Screenshaver adopts the
following compatibility contract:

> A database created by any publicly released Screenshaver schema must
> remain automatically migratable to every future supported Screenshaver
> schema without requiring the user to manually reconstruct policies,
> playlists, or other durable user data.

This contract has the following consequences:

1.  Durable user data and persisted user configuration must retain their
    semantic meaning across schema migrations.

2.  Migration compatibility concerns application semantics rather than
    the literal historical SQLite representation. Tables, columns,
    relationships, and internal IDs may change when a later schema
    represents the same durable information differently.

3.  Historical schema readers are permanent compatibility code. A reader
    must not be removed merely because its schema has become old.

4.  Representative historical database fixtures must be retained so that
    migration from every released schema can continue to be tested.

5.  Every future schema must be tested for migration from every
    historical schema covered by this compatibility contract.

6.  Migration failure must never cause automatic destruction,
    replacement, or reinitialization of the source database.

7.  The source database must remain untouched until a replacement
    database using the current schema has been completely constructed
    and successfully validated.

8.  If a proposed new schema cannot safely transform durable information
    from an older supported schema, the proposed schema must be
    redesigned rather than requiring the user to reconstruct that
    information manually.

9.  Physical shader source files are not to be deleted or modified
    merely as a consequence of database migration.

10. Corruption recovery and schema migration are separate concerns. A
    physically damaged database must not be treated as an ordinary
    migration failure.

------------------------------------------------------------------------

## Migration Architecture

Screenshaver migrations use a **reconstruction migration** model rather
than a permanent chain of sequential in-place SQL transformations.

The intended architecture is:

``` text
Historical Schema 1 ─► reader_v1 ─┐
Historical Schema 2 ─► reader_v2 ─┤
Historical Schema 3 ─► reader_v3 ─┤
...                               ├─► canonical MigrationData
Historical Schema N ─► reader_vN ─┘          │
                                             ▼
                                      current-schema writer
                                             │
                                             ▼
                                      current database
```

Each historical reader understands the semantics of one historical
schema and translates its durable information into a storage-independent
intermediate representation.

The current-schema writer creates the current database from scratch and
imports that intermediate representation.

`MigrationData` should describe Screenshaver application concepts rather
than SQLite implementation details so that migration logic is not
permanently coupled to SQLite.

### Migration Transaction Model

The conceptual migration sequence is:

``` text
screenshaver.db
      │
      ├── remains untouched
      ▼
Open source database READ-ONLY
      ↓
Identify source schema version
      ↓
Historical reader extracts durable information
      ↓
Create screenshaver.db.migrating
      ↓
Create CURRENT schema from scratch
      ↓
Populate current factory/developer data
      ↓
Import transformed durable user data
      ↓
Run SQLite integrity validation
      ↓
Run Screenshaver structural validation
      ↓
Run semantic/data validation
      ↓
SUCCESS?
  NO ─► remove staged database; retain original untouched; log diagnostics
 YES ─► atomic cutover; retain previous database temporarily as appropriate
```

Migration must never modify the historical source database in place.

------------------------------------------------------------------------

## Migration Defaults

Whenever a new schema introduces information that did not exist in an
older schema, the migration must explicitly determine how that
information is created.

The preferred decision order is:

1.  **Universal default** --- a value that is correct regardless of
    historical context.
2.  **Derived value** --- a value that can be reliably calculated from
    historical data.
3.  **Context-sensitive default** --- a value determined from the
    historical object's meaning or state.
4.  **Factory/developer value** --- current developer-maintained data
    recreated from the current executable.
5.  **Migration default** --- a value selected specifically to preserve
    the behavior of the older Screenshaver version when the historical
    user choice is unknowable.

A **creation default** and a **migration default** are different
concepts.

A creation default defines the behavior of a newly created object or
fresh installation.

A migration default defines the behavior assigned to an older object
that predates a new setting. Its purpose is to preserve historical
behavior as closely as practical.

The two values may legitimately differ.

------------------------------------------------------------------------

## Durable Identity and Internal IDs

SQLite integer primary keys are local implementation identities.

Within a database, Screenshaver treats allocated IDs as stable and does
not intentionally reuse retired IDs.

Across a reconstruction migration, however, numeric SQLite IDs are
**not** part of the compatibility guarantee. A destination database may
allocate different shader, policy, playlist, or relationship IDs.

Migration must instead preserve the semantic identity of objects and
reconstruct all relationships correctly.

For example:

-   a policy must continue to reference the same logical physical
    shader;
-   a playlist must continue to contain the same policies;
-   playlist ordering must be preserved;
-   a runtime target must continue to select the corresponding policy or
    playlist.

This permits future schemas to change internal database representation
without making historical numeric IDs part of the public compatibility
contract.

------------------------------------------------------------------------

## Metadata and Timestamps

Creation, modification, addition, and migration timestamps are useful
historical metadata but are not the primary semantic identity of
Screenshaver objects.

Historical timestamps should be retained during migration when practical
and meaningful, but byte-for-byte timestamp equality is not required for
migration correctness unless a future feature explicitly gives a
timestamp durable behavioral semantics.

Schema/application provenance metadata should be recreated appropriately
for the destination database while retaining sufficient information for
diagnostics and migration history.

------------------------------------------------------------------------

# Schema Version 1

## Status

**Released baseline / permanent historical schema**

## Introduced In

**Screenshaver 0.5.4**

## Introduction Date

**2026-09-26**

## Schema Definition

The authoritative Schema Version 1 structural definition is:

``` text
assets/database/schema_v001.sql
```

The schema is created by Screenshaver initialization code, which also
supplies runtime-dependent and developer-maintained initial data that
does not belong in the static SQL definition.

Schema Version 1 is the earliest database schema covered by
Screenshaver's permanent automatic-migration compatibility guarantee.

------------------------------------------------------------------------

## Schema 1 Design Principles

Schema Version 1 establishes the following application semantics:

-   Physical shaders and shader policies are separate objects.
-   One physical shader may be referenced by multiple policies.
-   Policy Name is the user-facing policy identifier.
-   Internal numeric IDs are database implementation identities.
-   Policy configuration is decomposed into typed semantic fields rather
    than consolidated configuration strings.
-   `NULL` represents inheritance only where the schema explicitly
    defines that meaning.
-   Physical shader files remain the authoritative shader source assets.
-   Derived shader runtime information is regenerable from physical
    shader source.
-   Playlist order is explicit durable data.
-   Runtime target selection is separate from policy definition.
-   Developer-maintained catalogs are distinct from user-created
    configuration.

------------------------------------------------------------------------

## Schema 1 Tables

Schema Version 1 contains ten application tables:

  -----------------------------------------------------------------------
  Table                   Primary classification  Purpose
  ----------------------- ----------------------- -----------------------
  `schema_metadata`       Provenance /            Identifies schema and
                          compatibility metadata  application-version
                                                  provenance

  `shaders`               Mixed durable and       Represents logical
                          derived data            physical shader assets
                                                  and their runtime
                                                  analysis

  `shader_policies`       Durable user data       Stores named shader
                                                  configurations

  `playlists`             Durable user data       Stores user-defined
                                                  ordered policy
                                                  collections

  `playlist_members`      Durable user            Stores playlist
                          relationships           membership and
                                                  authoritative order

  `runtime_targets`       Durable local           Stores Screensaver and
                          configuration           Wallpaper selection
                                                  modes

  `app_defaults`          Durable local           Stores application-wide
                          configuration           persisted defaults

  `target_defaults`       Durable local           Stores Screensaver- and
                          configuration           Wallpaper-specific
                                                  inherited defaults

  `textures`              Factory/developer       Stores texture families
                          catalog                 supported by the
                                                  current build

  `curated_palette`       Factory/developer       Stores the current
                          catalog                 developer-maintained
                                                  palette catalog
  -----------------------------------------------------------------------

------------------------------------------------------------------------

## `schema_metadata`

Schema 1 permits exactly one metadata row.

Its principal fields are:

-   `schema_version`
-   `created_by_version`
-   `last_migrated_by_version`

This table identifies the structural contract and records application
provenance.

During reconstruction migration, destination provenance must describe
the newly created destination database. Historical provenance may be
retained separately when useful for diagnostics, but old provenance
values must not cause the destination database to misidentify its
current schema.

------------------------------------------------------------------------

## `shaders`

A Schema 1 shader row represents one logical physical shader asset.

### Durable physical identity

The durable physical-asset information is centered on:

-   `filename`
-   `source_path`

The combination of source directory and filename identifies the
registered physical pathname in Schema 1.

Physical shader source files remain authoritative.

### Observed and regenerable shader state

The following values describe the source file or the result of current
analysis and may be recalculated when appropriate:

-   `shader_type`
-   `source_hash`
-   `file_status`
-   `validation_status`
-   `validation_reason`
-   `validation_message`

Migration should preserve useful historical information when
appropriate, but current source inspection and validation may supersede
stale observations.

### Derived runtime package

The following four fields form the Schema 1 derived runtime package:

-   `preprocessed_source`
-   `preprocessor_version`
-   `channel_usage_mask`
-   `shader_inputs_json`

These values are derived from the authoritative shader source and the
Screenshaver runtime-source preparation contract.

A future migration should not assume that an old preprocessed runtime
package remains valid under the current preprocessor. The current
application may regenerate it from the physical source using the current
preparation contract.

------------------------------------------------------------------------

## `shader_policies`

Shader policies are core durable user data.

A policy represents a named rendition/configuration of a physical
shader.

Migration must preserve the semantic meaning of the policy, including as
applicable:

-   Policy Name;
-   physical shader relationship;
-   Policy Target;
-   texture behavior;
-   palette behavior;
-   rendered FPS override;
-   animation speed;
-   starting offset;
-   anti-aliasing;
-   dithering;
-   color precision;
-   render scale;
-   audiovisual effect;
-   Audio Motion effect;
-   bloom parameters;
-   color inversion;
-   horizontal and vertical flipping;
-   hue rotation.

The compatibility promise applies to the **meaning** of these values
rather than requiring future schemas to retain the same columns.

Schema 1 Policy Targets use the stable internal values:

``` text
unassigned
screensaver
wallpaper
```

These values are application identifiers and are not user-interface
translation strings.

### Policy Name

`policy_name` preserves the user-visible spelling and capitalization.

`policy_name_key` is a comparison/search representation generated by
Screenshaver. It is not the fundamental policy identity and may be
regenerated under a future canonicalization contract if a migration
explicitly defines that transformation.

------------------------------------------------------------------------

## `playlists`

Playlists are durable user-created objects.

Migration must preserve:

-   playlist name;
-   optional description;
-   membership;
-   member order.

Playlist names are represented separately from their normalized
comparison keys.

------------------------------------------------------------------------

## `playlist_members`

`playlist_members` represents the durable ordered many-to-many
relationship between playlists and shader policies.

`position` is the authoritative persistent playlist order.

Migration must reconstruct membership and ordering by semantic object
relationships rather than by assuming that historical numeric policy or
playlist IDs remain unchanged.

------------------------------------------------------------------------

## `runtime_targets`

`runtime_targets` stores durable local selection configuration for:

``` text
screensaver
wallpaper
```

Schema 1 supports these display modes:

``` text
single
ordered
random
playlist
```

Migration must preserve the effective runtime selection behavior where
possible, including:

-   display mode;
-   rotation interval;
-   selected Single-mode policy;
-   selected Playlist-mode playlist.

Runtime target configuration is durable configuration and must not be
confused with ephemeral playback position.

------------------------------------------------------------------------

## `app_defaults`

`app_defaults` contains durable application-wide persisted
configuration.

Schema 1 includes settings such as:

-   splash-screen behavior;
-   screensaver subtitle behavior and placement;
-   wallpaper notifications;
-   synchronized-lyrics enablement;
-   wallpaper display format;
-   rendered FPS;
-   anti-aliasing;
-   dithering;
-   color precision;
-   render scale;
-   automatic-backup behavior and interval.

Existing user choices should be preserved across migrations.

When a later schema adds a setting that did not exist in Schema 1, its
historical value must be determined using the migration-default rules
defined earlier in this document.

Some startup/recovery settings intentionally remain outside the database
in `screenshaver.toml`; Schema 1 does not make the database the sole
owner of all Screenshaver configuration.

------------------------------------------------------------------------

## `target_defaults`

`target_defaults` stores durable inherited defaults separately for
Screensaver and Wallpaper.

Schema 1 includes:

-   Screensaver idle timeout magnitude and unit;
-   animation speed;
-   texture mode;
-   texture family;
-   texture primitive count;
-   palette mode;
-   palette color.

User-selected values are durable configuration and must survive
migration.

Schema 1 deliberately stores the Screensaver idle timeout magnitude and
unit rather than flattening the value to seconds.

------------------------------------------------------------------------

## `textures`

`textures` is a developer-maintained factory catalog.

It is seeded from the texture families supported by the Screenshaver
executable.

The catalog itself is not user-created durable data and should normally
be recreated from the current executable during reconstruction
migration.

Any durable policy meaning that refers to a texture family must
nevertheless be transformed safely. Recreating the catalog must not
silently change the meaning of an existing policy.

------------------------------------------------------------------------

## `curated_palette`

`curated_palette` is a developer-maintained reference catalog.

Policies do not reference palette rows by database ID. A policy using a
specific palette color stores the hexadecimal color value itself.

Therefore changes to the developer-maintained curated catalog do not
automatically change existing policy colors.

During reconstruction migration, the catalog should normally be
recreated from the current executable.

------------------------------------------------------------------------

## Factory Objects

A fresh Schema 1 database includes factory-created data such as:

-   the developer-maintained texture catalog;
-   the developer-maintained curated palette;
-   application defaults;
-   Screensaver and Wallpaper target defaults;
-   `default.glsl`;
-   a `screensaver default` policy;
-   a `wallpaper default` policy;
-   initial Screensaver and Wallpaper runtime-target selections.

Future reconstruction migrations should distinguish between:

1.  factory/developer objects that should be recreated according to the
    current executable; and
2.  durable user configuration that may have originated from factory
    defaults but was subsequently selected or changed by the user.

"Factory-created" must not be interpreted as permission to discard a
user's persisted configuration.

------------------------------------------------------------------------

## Transient State Outside Schema 1

Not all Screenshaver state belongs in the database.

Operational UI/runtime state may remain in `state.json`.

For example, Ordered-mode continuation state is runtime state rather
than durable playlist or policy definition.

Future schema work should continue to distinguish:

-   durable user information;
-   persisted local configuration;
-   factory/developer catalogs;
-   derived/regenerable data;
-   transient runtime/UI state.

A database schema should not absorb transient state merely because
storing it in SQLite is convenient.

------------------------------------------------------------------------

## Schema 1 Migration Requirements

Any future historical Schema-1 reader must be able to extract enough
semantic information to reconstruct, in the then-current schema:

1.  user physical-shader registrations and their policy relationships;
2.  all durable shader policies;
3.  all user playlists;
4.  playlist memberships and ordering;
5.  Screensaver and Wallpaper runtime-target configuration;
6.  application defaults that represent persisted user choices;
7.  target defaults that represent persisted user choices.

The reader need not treat local numeric IDs, developer catalogs, or
derived runtime shader packages as portable durable values.

A Schema-1 source database must be opened read-only during
reconstruction migration and must remain unchanged unless and until a
separately constructed current-schema database has passed all required
validation and an atomic cutover is performed.

------------------------------------------------------------------------

## Schema 1 Historical Fixture

Before Schema 2 is released, the project should preserve at least one
representative Schema-1 database fixture suitable for automated
migration testing.

A useful fixture should contain more than factory defaults. It should
exercise, at minimum:

-   multiple physical shaders;
-   multiple policies referencing at least one common shader;
-   all Policy Targets;
-   inherited and explicit policy settings;
-   representative audiovisual and Audio Motion effects;
-   at least two playlists;
-   playlist membership and ordering;
-   each runtime-target display mode where practical;
-   non-default application settings;
-   non-default Screensaver and Wallpaper target defaults.

The fixture must not depend on private user data or irreplaceable local
files.

Once established as a historical migration fixture, it should be treated
as immutable test input.

------------------------------------------------------------------------

# Future Schemas

Each publicly released schema should receive a permanent section in this
document.

At minimum, each section should record:

-   schema version;
-   introducing Screenshaver version;
-   introduction date;
-   authoritative schema-definition file;
-   structural changes;
-   semantic changes;
-   new durable fields;
-   removed or transformed fields;
-   creation defaults;
-   migration defaults;
-   factory-data changes relevant to migration;
-   historical-reader implementation;
-   migration validation requirements;
-   any special compatibility considerations.

------------------------------------------------------------------------

# Schema Version 2

## Status

**Not yet released / design not frozen**

## Planned Introducing Version

**Screenshaver 0.5.5**

The application version must be incremented from 0.5.4 to 0.5.5 before
compiling the first Screenshaver code that actually introduces Database
Schema Version 2.

The exact Schema 2 definition must not be recorded as final until its
structural design is approved.

Localization support is currently expected to provide the first
practical Schema 1 → Schema 2 reconstruction-migration exercise, but
this document deliberately does not freeze Schema 2's final structure in
advance.

------------------------------------------------------------------------

## Schema History Summary

  -----------------------------------------------------------------------------
             Schema Screenshaver   Introduction   Status        Summary
                    version        date                         
  ----------------- -------------- -------------- ------------- ---------------
                  1 0.5.4          2026-09-26     Baseline /    First permanent
                                                  released      compatibility
                                                                baseline;
                                                                policies,
                                                                shaders,
                                                                playlists,
                                                                runtime
                                                                targets,
                                                                defaults, and
                                                                developer
                                                                catalogs

                  2 0.5.5          TBD            Not released  Structure not
                    (planned)                                   yet frozen
  -----------------------------------------------------------------------------

------------------------------------------------------------------------

## Maintenance Rule

This document must be updated as part of every database schema-version
change.

A schema change is not considered complete until:

1.  its schema definition is frozen;
2.  its section in this history is complete;
3.  its historical reader/fixture requirements are satisfied;
4.  migration from every previously supported released schema has been
    tested;
5.  the Screenshaver application version has been incremented as
    required;
6.  migration failure behavior has been verified to leave the source
    database intact.

Changes that alter only developer-maintained catalog data, factory
content, translations, or application defaults do not automatically
require a schema-version increment when the database structure and
durable semantic contract remain compatible. Such changes should still
be documented when they materially affect migration or historical
interpretation.
