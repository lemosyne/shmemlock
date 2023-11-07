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
#include <stdint.h>

struct shm {
    pthread_mutex_t lock;
    uint64_t magic;
};

int main(int argc, char **argv) {
    size_t len = sizeof(struct shm);
    char *path = "/leo.psm";
    void *base_addr_req = (void *)0x200000000000;

    if (argc < 2) {
        unlink("/dev/shm/leo.psm");
    }

    int fd = shm_open(path, O_RDWR | O_CREAT, 0660);
    if (fd < 0) {
        err(EXIT_FAILURE, "%s (%zu)", path, len);
    }

    int res = ftruncate(fd, len);
    if (res != 0) {
        err(EXIT_FAILURE, NULL);
    }


    struct shm *shm = mmap(NULL, len, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    if (shm == MAP_FAILED) {
        err(EXIT_FAILURE, "lol");
    }

    printf("%lx\n", shm->magic);

    if (argc < 2) {
        pthread_mutexattr_t attr;
        pthread_mutexattr_init(&attr);

        assert(pthread_mutexattr_setpshared(&attr, PTHREAD_PROCESS_SHARED) == 0);
        assert(pthread_mutex_init(&shm->lock, &attr) == 0);

        shm->magic = 0x4141414141414141;
    }

    for (;;) {
        assert(pthread_mutex_lock(&shm->lock) == 0);

        printf("start = %ld\n", time(NULL));
        sleep(3);
        printf("end = %ld\n", time(NULL));

        assert(pthread_mutex_unlock(&shm->lock) == 0);

        sleep(3);
    }

    return 0;
}
