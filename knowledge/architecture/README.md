# architecture/

Pages split out of [`../ARCHITECTURE.md`](../ARCHITECTURE.md), one topic each. That page is
the entry point and keeps the section numbers that code and ADRs cite.

| Page | § | Holds | Changes when |
| --- | --- | --- | --- |
| [`pipeline.md`](pipeline.md) | 5 | The extraction sequence, the Tier-1 gate, both Tier-2 paths, concurrency, outputs, the server's routes | the pipeline is restructured |
| [`configuration.md`](configuration.md) | 12 | Every environment variable, and the CLI's exit codes | a variable or exit code is added, changed or removed |

**What belongs here:** a description of the system as it is today, for one topic that changes
on its own schedule.

**What does not:** history (it goes in `CHANGELOG.md`), measured numbers
(`../benchmarks/README.md`), decisions and their rejected alternatives (`../decisions/`), and
policy every contributor must read (`../ARCHITECTURE.md` §13).
