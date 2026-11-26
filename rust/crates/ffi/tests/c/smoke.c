/* C smoke test of the kod_ffi static library (issue #7).
 * Built against the generated header exactly as Unreal consumes it:
 * roundtrip, ownership, error paths, panic containment, worker threads. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "kod_ffi.h"

#ifdef _WIN32
#include <windows.h>
typedef HANDLE thread_t;
#define THREAD_FN DWORD WINAPI
#define THREAD_RET return 0
static void start_thread(thread_t *t, LPTHREAD_START_ROUTINE fn, void *arg) { *t = CreateThread(NULL, 0, fn, arg, 0, NULL); }
static void join_thread(thread_t t) { WaitForSingleObject(t, INFINITE); CloseHandle(t); }
#else
#include <pthread.h>
typedef pthread_t thread_t;
#define THREAD_FN void *
#define THREAD_RET return NULL
static void start_thread(thread_t *t, void *(*fn)(void *), void *arg) { pthread_create(t, NULL, fn, arg); }
static void join_thread(thread_t t) { pthread_join(t, NULL); }
#endif

/* Exported only with the `test-hooks` feature; not part of the public header. */
enum KodStatus kod_test_panic(struct KodSim *sim);

static int failures = 0;
#define CHECK(cond)                                                     \
    do {                                                                \
        if (!(cond)) {                                                  \
            fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond); \
            failures++;                                                 \
        }                                                               \
    } while (0)

static const KodDate START = {1836, 1, 1};

static KodSim *create(uint64_t seed) {
    KodSim *sim = NULL;
    CHECK(kod_sim_create(KOD_ABI_VERSION, seed, START, &sim) == KOD_STATUS_OK);
    return sim;
}

static KodHash128 hash_of(KodSim *sim) {
    KodHash128 h = {0, 0};
    CHECK(kod_sim_state_hash(sim, &h) == KOD_STATUS_OK);
    return h;
}

static int same_hash(KodHash128 a, KodHash128 b) { return a.lo == b.lo && a.hi == b.hi; }

static void test_roundtrip(void) {
    KodSim *sim = create(42);
    const char *name = "Staatskasse";
    CHECK(kod_ledger_open(sim, 1, 1, 0, (const uint8_t *)name, strlen(name), 100000000000LL) == KOD_STATUS_OK);
    CHECK(kod_ledger_open(sim, 2, 1, 0, (const uint8_t *)"B", 1, 5000000000LL) == KOD_STATUS_OK);
    CHECK(kod_ledger_transfer(sim, 2, 2, 1, 1, 2, 1) == KOD_STATUS_OK); /* unauthorized at execution */

    KodStepSummary s;
    CHECK(kod_sim_step(sim, 2, &s) == KOD_STATUS_OK);
    CHECK(s.next_tick == 2 && s.applied == 2 && s.rejected == 1);
    KodRejection rej[2];
    size_t n = 0;
    CHECK(kod_sim_rejections(sim, rej, 2, &n) == KOD_STATUS_OK);
    CHECK(n == 1 && rej[0].actor == 2 && rej[0].sequence == 2 && rej[0].code == 1);

    size_t len = 0;
    CHECK(kod_sim_save(sim, NULL, 0, &len) == KOD_STATUS_BUFFER_TOO_SMALL && len > 0);
    uint8_t *buf = (uint8_t *)malloc(len);
    CHECK(kod_sim_save(sim, buf, len, &len) == KOD_STATUS_OK);
    KodSim *loaded = NULL;
    CHECK(kod_sim_load(KOD_ABI_VERSION, buf, len, &loaded) == KOD_STATUS_OK);
    CHECK(same_hash(hash_of(sim), hash_of(loaded)));
    buf[len / 2] ^= 0x40;
    KodSim *corrupt = NULL;
    CHECK(kod_sim_load(KOD_ABI_VERSION, buf, len, &corrupt) == KOD_STATUS_INVALID_SAVE && corrupt == NULL);
    free(buf);

    CHECK(kod_sim_step(sim, 365, NULL) == KOD_STATUS_OK);
    CHECK(kod_sim_step(loaded, 365, NULL) == KOD_STATUS_OK);
    CHECK(same_hash(hash_of(sim), hash_of(loaded)));
    KodDate d;
    CHECK(kod_sim_date(sim, &d) == KOD_STATUS_OK && d.year == 1837 && d.month == 1 && d.day == 2);
    kod_sim_destroy(sim);
    kod_sim_destroy(loaded);
}

static void test_errors(void) {
    KodSim *sim = NULL;
    CHECK(kod_sim_create(KOD_ABI_VERSION + 1, 1, START, &sim) == KOD_STATUS_ABI_MISMATCH && sim == NULL);
    char msg[256];
    size_t len = 0;
    CHECK(kod_last_error((uint8_t *)msg, sizeof msg, &len) == KOD_STATUS_OK && len > 0);
    CHECK(kod_sim_step(NULL, 1, NULL) == KOD_STATUS_NULL_POINTER);
    kod_sim_destroy(NULL);

    sim = create(5);
    CHECK(kod_test_panic(sim) == KOD_STATUS_PANIC);
    CHECK(kod_sim_step(sim, 1, NULL) == KOD_STATUS_POISONED);
    kod_sim_destroy(sim);
}

static KodHash128 worker_results[4];

static THREAD_FN worker(void *arg) {
    size_t idx = (size_t)arg;
    KodSim *sim = create(77);
    for (uint64_t seq = 1; seq <= 50; seq++) {
        char name[16];
        int n = snprintf(name, sizeof name, "acct-%u", (unsigned)(seq % 7));
        kod_ledger_open(sim, 1, seq, seq, (const uint8_t *)name, (size_t)n, 1000000000LL);
    }
    kod_sim_step(sim, 365, NULL);
    worker_results[idx] = hash_of(sim);
    kod_sim_destroy(sim);
    THREAD_RET;
}

static void test_threads(void) {
    thread_t t[4];
    for (size_t i = 0; i < 4; i++) start_thread(&t[i], worker, (void *)i);
    for (size_t i = 0; i < 4; i++) join_thread(t[i]);
    for (size_t i = 1; i < 4; i++) CHECK(same_hash(worker_results[0], worker_results[i]));
}

int main(void) {
    CHECK(kod_abi_version() == KOD_ABI_VERSION);
    test_roundtrip();
    test_errors();
    test_threads();
    if (failures) {
        fprintf(stderr, "kod_ffi C smoke test: %d failure(s)\n", failures);
        return 1;
    }
    printf("kod_ffi C smoke test: ok (abi %u, hash %016llx%016llx)\n", kod_abi_version(),
           (unsigned long long)worker_results[0].hi, (unsigned long long)worker_results[0].lo);
    return 0;
}
