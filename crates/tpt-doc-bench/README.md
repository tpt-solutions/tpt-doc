# tpt-doc-bench

Criterion benchmarks for the format crates. It measures parsing, writing and
round-trip speed on fixed fixtures, so a change that slows a path shows up as a
number rather than a feeling.

- **Status:** unpublished (`publish = false`). MSRV 1.88. Licence `MIT OR Apache-2.0`.

## Benchmark groups

| Group | What it measures |
|---|---|
| `hl7_parse` | Parsing an ADT^A01 message |
| `edifact_parse` | Parsing an EDIFACT interchange |
| `xlsx_round_trip` | Writing and reading a 1000 by 10 workbook |
| `pdf_write` | Writing a 50-line PDF |
| `fhir_patient` | Serializing and parsing a FHIR Patient |

## Recent results

These were measured on the development machine and are indicative only. They
will vary with hardware and with the compiler version.

| Benchmark | Typical time |
|---|---|
| HL7 ADT^A01 parse | about 0.6 µs |
| EDIFACT parse | about 0.8 µs |
| XLSX 1000 by 10 write and read | about 12.6 ms |
| 50-line PDF write | about 46 µs |
| FHIR Patient JSON | about 0.8 µs |

## Running

```sh
cargo bench -p tpt-doc-bench
```

Criterion stores its reports in `target/criterion/`. For a quick check that the
benchmarks still build without running them, use:

```sh
cargo bench -p tpt-doc-bench --no-run
```

## Licence

Licensed under either of Apache-2.0 or MIT, at your option.
