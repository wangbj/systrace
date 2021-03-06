#include <sys/types.h>
#include <time.h>
#include <stdlib.h>
#include <stdio.h>
#include <errno.h>
#include <string.h>

int main(int argc, char* argv[])
{
  struct timespec req = {
    .tv_sec = 0,
    .tv_nsec = 100000000,
  };
  struct timespec rem;
  int ret;
  
  do {
    ret = clock_nanosleep(CLOCK_REALTIME, 0, &req, &rem);
    memcpy(&req, &rem, sizeof(req));
  } while (ret != 0 && errno == EINTR);

  return 0;
}

