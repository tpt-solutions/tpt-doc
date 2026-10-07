# Security Policy

tpt-doc parses untrusted documents (PDF, OOXML, HL7, EDI, XML) and produces digital signatures, so security reports are taken seriously.

## Reporting a vulnerability

Please report privately via GitHub's **Security → Report a vulnerability** on this repository. Include the crate, version, and a minimal reproducing input. Do not file public issues for vulnerabilities.

You can expect an acknowledgement within 5 working days. Fixes are released as patch versions with a RustSec advisory where applicable.

## Scope

In scope: panics, unbounded memory/CPU on crafted input, signature-validation or signing flaws, XML/ZIP bomb handling. Only the latest `0.x` release is supported.
