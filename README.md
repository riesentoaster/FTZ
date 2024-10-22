# Fuzzing Zephyr's Network Stack

Using LibAFL, I want to fuzz the TCP/IP stack of Zephyr.

## Communication Protocol
- shmem\[`offset`\]:     Size, negative for ready
- shmem\[`offset+1..`\]: Data

Shared Memory is split in two such sub-buffers for the two directions, where `offset`:
- `0` for the packets going from the fuzzer to the system under test
- `shmem_len/2` for packets going from the SUT to the fuzzer

