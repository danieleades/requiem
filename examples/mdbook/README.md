# MdBook

Requiem can be used alongside MdBook. The only requirement is that files which are not requirements do not use filenames which have the same syntax as a Requiem human readable ID (HRID).

## Generated Navigation

`src/SUMMARY.md` contains a marker-delimited section that Requiem keeps in sync with the requirements on disk:

```markdown
<!-- requiem:summary:start -->
<!-- requiem:summary:end -->
```

Regenerate it after adding, renaming, or removing requirements:

```sh
req --root src export summary
```

Content outside the markers is never touched, so requirements can share the summary with hand-written chapters. In CI, `req --root src export summary --check` fails the build if the navigation has drifted.

## Frontmatter

MdBook does not parse YAML frontmatter natively, so this example uses the [`mdbook-yml-header`](https://crates.io/crates/mdbook-yml-header) preprocessor to strip it from rendered pages.

## Building the Example

first install MdBook and the frontmatter preprocessor:

```sh
cargo install mdbook mdbook-yml-header
```

then build the book

```sh
mdbook build
```
