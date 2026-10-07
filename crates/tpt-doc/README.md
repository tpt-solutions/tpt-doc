# tpt-doc

Facade crate for the [tpt-doc](https://github.com/tpt-solutions/tpt-doc) workspace. Enable the formats you need:

```toml
[dependencies]
tpt-doc = { version = "0.1", default-features = false, features = ["fhir", "hl7v2"] }
```

Each feature re-exports its crate as a module (`tpt_doc::fhir`, `tpt_doc::pdf`, …). Heavy dependencies (`ring`, `x509-parser`, `zip`) are only compiled when `sign`, `spreadsheet` or `word` is enabled. `full` turns everything on.
