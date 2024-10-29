#include <stdio.h>
#include <stdint.h>

// #define LOG_COV 1

static uint32_t *guard_stop = 0;
static uint32_t *guard_start = 0;

void __sanitizer_cov_trace_pc_guard_init(uint32_t *start, uint32_t *stop)
{
  if (start == stop)
  {
#ifdef LOG_COV
    fprintf(stderr, "Skipping initialization");
#endif
    return;
  };

#ifdef LOG_COV
  fprintf(stderr, "Initializing called with start %p and stop %p\n", start,
          stop);
#endif
  guard_start = start;
  guard_stop = stop;

  for (uint32_t *x = start; x < stop; x++)
    *x = 0;

#ifdef LOG_COV
  fprintf(stderr, "Done with initialization\n");
#endif
}

void __sanitizer_cov_trace_pc_guard(uint32_t *guard)
{
  *guard += 1;
  // *guard = 1;
#ifdef LOG_COV
  fprintf(stderr, "Updated guard %p to %u\n", guard, *guard);
#endif
}

__attribute__((visibility("default"))) size_t get_guard_count()
{
#ifdef LOG_COV
  fprintf(stderr, "Returned guard count %zu\n", guard_stop - guard_start);
#endif
  return guard_stop - guard_start;
}

__attribute__((visibility("default"))) uint32_t *get_guard_values()
{
#ifdef LOG_COV
  fprintf(stderr, "Returned guard values %p\n", guard_start);
#endif
  return guard_start;
}