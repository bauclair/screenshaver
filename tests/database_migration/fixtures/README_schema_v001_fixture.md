# Screenshaver Schema-1 Historical Migration Fixture

`schema_v001.db` is the permanent representative historical fixture for
Screenshaver Database Schema Version 1, introduced in Screenshaver 0.5.4.

The fixture is synthetic. It contains no private user data and does not depend
on real shader files. Its purpose is to provide immutable historical input for
migration regression testing.

## Contents

The fixture contains:

- 3 physical shader registrations;
- 6 shader policies;
- all three policy targets: `screensaver`, `wallpaper`, and `unassigned`;
- multiple policies referencing the same physical shader;
- inherited, random, and specific texture/palette policy behavior;
- all Schema-1 audiovisual-effect values;
- all Schema-1 Audio Motion effect values;
- representative render overrides, transforms, timestamps, and starting offsets;
- 2 playlists with explicit persistent ordering;
- runtime `single` and `playlist` modes with live foreign-key references;
- deliberately non-default application defaults;
- deliberately non-default Screensaver and Wallpaper target defaults;
- minimal developer texture/palette catalog rows; and
- Schema-1 provenance identifying Screenshaver 0.5.4.

A single Schema-1 database can store only one display mode for each of its two
runtime targets. This fixture therefore cannot simultaneously encode all four
`single`, `ordered`, `random`, and `playlist` modes. It uses `single` and
`playlist` because those modes exercise policy/playlist foreign-key references.
Alternate runtime-mode cases should be exercised by automated tests using
throwaway copies rather than by modifying this permanent fixture.

## Authoritative structure

The database structure must come from the frozen project file:

```text
assets/database/schema_v001.sql
```

`schema_v001_fixture_seed.sql` contains only the deterministic fixture data.
It must not become a second independent definition of Schema 1.

## Rebuilding the fixture

From the Screenshaver project root, using a SQLite command-line client:

```bash
rm -f tests/database_migration/fixtures/schema_v001.db
sqlite3 tests/database_migration/fixtures/schema_v001.db \
    < assets/database/schema_v001.sql
sqlite3 tests/database_migration/fixtures/schema_v001.db \
    < tests/database_migration/fixtures/schema_v001_fixture_seed.sql
```

Then verify physical integrity and foreign keys:

```bash
sqlite3 tests/database_migration/fixtures/schema_v001.db \
    'PRAGMA integrity_check; PRAGMA foreign_key_check;'
```

Expected `integrity_check` result:

```text
ok
```

`foreign_key_check` should produce no rows.

Finally, run the Screenshaver Schema-1 historical reader against the fixture as
part of the migration regression tests. The checked-in `schema_v001.db` should
be treated as immutable historical input once accepted.

## Expected semantic inventory

- schema version: 1
- created by Screenshaver: 0.5.4
- last migrated by Screenshaver: 0.5.4
- shaders: 3
- policies: 6
  - screensaver: 3
  - wallpaper: 2
  - unassigned: 1
- playlists: 2
- playlist memberships: 7
- Screensaver runtime: single, policy `Fixture Alpha Alternate`
- Wallpaper runtime: playlist `Fixture Mixed Rotation`, interval 37 seconds

The application and both target-default records must also be successfully
extracted by `read_schema_v001`.
