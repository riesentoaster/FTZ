# Fuzzing Zephyr's Network Stack

Using LibAFL, I want to fuzz the TCP/IP stack of Zephyr.

## Repo management

This repository uses [git lfs](https://git-lfs.com) for storing large log files and [precommit](https://pre-commit.com) to ensure some consistency. Install as necessary. 

It also includes LibAFL as a git submodule to allow using the current version. Clone the repository using `git clone --recurse-submodules https://github.com/riesentoaster/fuzzing-zephyr-network-stack`.

## Report

Read the report [here](./report/out/index.pdf). Its artifacts are in the [`report`](./report/) subdirectory. The report is licensed under [CC BY-NC-ND 4.0](https://creativecommons.org/licenses/by-nc-nd/4.0/).

## Fuzzer

The code for the fuzzer can be found in the [`fuzzer`](./fuzzer/) subdirectory. The fuzzer is released under an MIT license.

### Environment

This project relies on a default installation of Zephyr relative to this folder at `../zephyrproject/zephyr`. The Python virtual environment should be placed at `../zephyrproject/.venv`.

#### Zephyr Diff

Changes to Zephyr are stored in [zephyr.diff](./zephyr.diff). It is updated on each commit using [pre-commit](https://pre-commit.com). Apply it using `git apply`. It is based on commit `8fda052826d`.

### Communication Protocol/Custom Layer 1

This project uses a custom OSI Layer 1 implementation based on shared memory to reduce performance implications on kernel interactions and make multiple parallel instances possible. Per default, the `native_sim` wrapper of Zephyr relies on a TUN interface, which only one process can use. With this custom implementation, only a single kernel interaction is necessary to setup the shared memory. Here is how the shared memory is used:

- shmem\[`offset`\]:     Size, negative for ready
- shmem\[`offset+1..`\]: Data

Shared Memory is split in two such sub-buffers for the two directions, where `offset`:
- `0` for the packets going from the fuzzer to the system under test
- `shmem_len/2` for packets going from the SUT to the fuzzer

The environment variables `SHMEM_ETH_INTERFACE_NAME` and `SHMEM_ETH_INTERFACE_SIZE` are used to communicate the necessary information to the SUT.
