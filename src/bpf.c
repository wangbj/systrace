#include <sys/types.h>
#include <sys/syscall.h>
#include <sys/personality.h>
#include <sys/prctl.h>
#include <linux/seccomp.h>
#include <linux/filter.h>
#include <linux/audit.h>
#include <unistd.h>
#include <stddef.h>
#include <assert.h>

#include "bpf-helper.h"

void bpf_install(void)
{
  struct bpf_labels l = {
    .count = 0,
  };
  struct sock_filter filter[] = {
    LOAD_SYSCALL_NR,
    SYSCALL(__NR_clone, ALLOW),
    SYSCALL(__NR_fork, ALLOW),
    SYSCALL(__NR_vfork, ALLOW),
    SYSCALL(__NR_rt_sigreturn, ALLOW),
    SYSCALL(__NR_clock_nanosleep, ALLOW),	// this syscall should not be patched
    LOAD_SYSCALL_IP,
    IP(0x70000002, ALLOW),
    TRACE,
  };
  struct sock_fprog prog = {
    .filter = filter,
    .len = (unsigned short)(sizeof(filter)/sizeof(filter[0])),
  };

  bpf_resolve_jumps(&l, filter, sizeof(filter)/sizeof(*filter));

  assert(prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &prog) == 0);
}
