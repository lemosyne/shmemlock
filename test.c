#include <assert.h>
#include <err.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>
#include <pthread.h>
#include <time.h>

int main(int argc, char **argv) {
    size_t len = 0x100000;
    char *path = "leo.psm";
    void *base_addr_req = (void *)0x200000000000;

    int fd = shm_open(path, O_RDWR | O_CREAT, S_IRUSR | S_IWUSR);
    if (fd < 0) {
        err(EXIT_FAILURE, "%s (%zu)", path, len);
    }

    int res = ftruncate(fd, len);
    if (res != 0) {
        err(EXIT_FAILURE, NULL);
    }

    int flags = MAP_SHARED_VALIDATE | MAP_FIXED;

    pthread_mutex_t *lock = mmap(base_addr_req, len, PROT_READ | PROT_WRITE, flags, fd, 0);
    if (lock == MAP_FAILED) {
        err(EXIT_FAILURE, "lol");
    }

    if (argc < 2) {
        pthread_mutexattr_t attr;
        pthread_mutexattr_init(&attr);

        assert(pthread_mutexattr_setpshared(&attr, PTHREAD_PROCESS_SHARED) == 0);
        assert(pthread_mutexattr_setrobust(&attr, PTHREAD_MUTEX_ROBUST) == 0);
        assert(pthread_mutex_init(lock, &attr) == 0);
    }

    for (;;) {
        assert(pthread_mutex_lock(lock) == 0);

        printf("start = %ld\n", time(NULL));
        sleep(3);
        printf("end = %ld\n", time(NULL));
        sleep(3);

        assert(pthread_mutex_unlock(lock) == 0);
    }

    return 0;
}
