## Stateful Targets

[SDFuzz: Target States Driven Directed
Fuzzing](https://www.usenix.org/conference/usenixsecurity24/presentation/li-penghui),
USENIX Security 2024
[Data Coverage for Guided
Fuzzing](https://www.usenix.org/conference/usenixsecurity24/presentation/wang-mingzhe),
USENIX Security 2024
* Approach: Extract dictionary values during runtime
[SandPuppy: Deep-State Fuzzing Guided by Automatic Detection of State-Representative Variables](https://link.springer.com/chapter/10.1007/978-3-031-64171-8_12),
DIMVA'24
* Target: User-space programs
* Data coverage metric: Automatically identified state variables
[DatAFLow: Toward a Data-Flow-Guided
Fuzzer](https://dl.acm.org/doi/10.1145/3587156), ACM Transactions on Software Engineering and Methodology, Volume 32, Issue 5, 2023
* Data coverage metric: Def-Use-Chains
[Fuzzing with Data Dependency
Information](https://ieeexplore.ieee.org/document/9797358), EuroS&P'22
* Data coverage metric: Def-Use-Chains
[StateAFL: Greybox fuzzing for stateful network servers](https://link.springer.com/article/10.1007/s10664-022-10233-3),
Empirical Software Engineering 2022
* Target: Network stack
* Data coverage metric: Specific state variables [Stateful Greybox Fuzzing](https://www.usenix.org/conference/usenixsecurity22/presentation/ba),
USENIX Security 2022
* Target: Network stack
* Data coverage metric: Specific state variables
[StateFuzz: System Call-Based State-Aware Linux Driver Fuzzing](https://www.usenix.org/conference/usenixsecurity22/presentation/zhao-bodong),
USENIX Security 2022
* Target: Linux kernel drivers
* Data coverage metric: Automatically identified state variables
[GREYONE: Data Flow Sensitive
Fuzzing](https://www.usenix.org/conference/usenixsecurity20/presentation/gan),
SEC'20
* Data coverage metric: Taint input bytes
[Ijon: Exploring Deep State Spaces via
Fuzzing](https://ieeexplore.ieee.org/document/9152719), S&P'20
* Data coverage metric: Manual annotation of state variables
* Target: User-space programs
[REDQUEEN: Fuzzing with Input-to-State
Correspondence](https://www.ndss-symposium.org/ndss-paper/redqueen-fuzzing-with-input-to-state-correspondence/),
NDSS'19
* Approach: Signal comparison values to fuzzer
[VUzzer: Application-aware Evolutionary Fuzzing](https://www.ndss-symposium.org/ndss2017/ndss-2017-programme/vuzzer-application-aware-evolutionary-fuzzing/),
NDSS'17
* Data coverage metric: Taint input bytes [Circumventing Fuzzing Roadblocks with Compiler Transformations](https://lafintel.wordpress.com/2016/08/15/circumventing-fuzzing-roadblocks-with-compiler-transformations/),
2016
* Approach: De-optimize code (e.g., comparisons)
