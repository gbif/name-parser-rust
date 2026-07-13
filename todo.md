# Backlog for things to do before we do a first full release

## API
- design the new 3 way parser interface

## test
- CI: rust + all bindings tested via GitHub Actions (.github/workflows/ci.yml); Jenkins stays
  scoped to the Java->Nexus deploy only. Validate the R + Java jobs on the first PR run.
- 

## docs
- document each binding
- document and justify the 3-way parse design with real examples for all the main cases, not just one for each kind. 
  Should give a user a clear guidance what to expect from the parser

## deployments fully configured
- test pypi
- setup R CRAN
