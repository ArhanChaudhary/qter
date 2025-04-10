#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <time.h>
#include <assert.h>

int branch = 0;
float count = 0;

#define solved_goto(b, label) \
    if (*b == 0)              \
    {                         \
        goto label;           \
    }

#define add(a, n) \
    {                         \
        *a += n;              \
        *a = (*a % 30 + 30) % 30; \
    }

void multiply(int *a, int *b, int *c)
{
    solved_goto(a, move_0_b);
    solved_goto(b, move_0_a);
reduce_by_2:
    solved_goto(b % 10, do_reduce_by_2);
    add(b, -1);
    add(c, 1);
    solved_goto(b % 10, before_reduce_by_3);
    add(b, -1);
    add(c, 1);
    goto reduce_by_2;
do_reduce_by_2:
reduce_by_1__3:
    solved_goto(c % 10, reduce_by_10__3);
    add(c, -1);
    add(b, 1);
    goto reduce_by_1__3;
reduce_by_10__3:
    solved_goto(c, after_move_loop__3);
    add(c, -10);
    add(b, 10);
    goto reduce_by_10__3;
after_move_loop__3:
reduce_b_loop__1:
    solved_goto(b, before_reduce_a_loop__1);
    add(b, -2);
    add(c, 1);
    goto reduce_b_loop__1;
before_reduce_a_loop__1:
reduce_by_1__1__1:
    solved_goto(c % 10, reduce_by_10__1__1);
    add(c, -1);
    add(b, 1);
    goto reduce_by_1__1__1;
reduce_by_10__1__1:
    solved_goto(c, after_move_loop__1__1);
    add(c, -10);
    add(b, 10);
    goto reduce_by_10__1__1;
after_move_loop__1__1:
reduce_a_loop__1:
    solved_goto(a, after_reduce_a_loop__1);
    add(a, -1);
    add(c, 2);
    goto reduce_a_loop__1;
after_reduce_a_loop__1:
reduce_by_1__2__1:
    solved_goto(c % 10, reduce_by_10__2__1);
    add(c, -1);
    add(a, 1);
    goto reduce_by_1__2__1;
reduce_by_10__2__1:
    solved_goto(c, after_move_loop__2__1);
    add(c, -10);
    add(a, 10);
    goto reduce_by_10__2__1;
after_move_loop__2__1:
    goto reduce_by_2;
do_reduce_by_3:
reduce_by_1__4:
    solved_goto(c % 10, reduce_by_10__4);
    add(c, -1);
    add(b, 1);
    goto reduce_by_1__4;
reduce_by_10__4:
    solved_goto(c, after_move_loop__4);
    add(c, -10);
    add(b, 10);
    goto reduce_by_10__4;
after_move_loop__4:
reduce_b_loop__2:
    solved_goto(b, before_reduce_a_loop__2);
    add(b, -3);
    add(c, 1);
    goto reduce_b_loop__2;
before_reduce_a_loop__2:
reduce_by_1__1__2:
    solved_goto(c % 10, reduce_by_10__1__2);
    add(c, -1);
    add(b, 1);
    goto reduce_by_1__1__2;
reduce_by_10__1__2:
    solved_goto(c, after_move_loop__1__2);
    add(c, -10);
    add(b, 10);
    goto reduce_by_10__1__2;
after_move_loop__1__2:
reduce_a_loop__2:
    solved_goto(a, after_reduce_a_loop__2);
    add(a, -1);
    add(c, 3);
    goto reduce_a_loop__2;
after_reduce_a_loop__2:
reduce_by_1__2__2:
    solved_goto(c % 10, reduce_by_10__2__2);
    add(c, -1);
    add(a, 1);
    goto reduce_by_1__2__2;
reduce_by_10__2__2:
    solved_goto(c, after_move_loop__2__2);
    add(c, -10);
    add(a, 10);
    goto reduce_by_10__2__2;
after_move_loop__2__2:
    goto reduce_by_3;
before_reduce_by_3:
reduce_by_1__5:
    solved_goto(c % 10, reduce_by_10__5);
    add(c, -1);
    add(b, 1);
    goto reduce_by_1__5;
reduce_by_10__5:
    solved_goto(c, after_move_loop__5);
    add(c, -10);
    add(b, 10);
    goto reduce_by_10__5;
after_move_loop__5:
reduce_by_3:
    solved_goto(b % 3, do_reduce_by_3);
reduce_by_5:
    solved_goto(b % 10, do_reduce_by_5);
    add(b, -1);
    add(c, 1);
    solved_goto(b % 10, before_reduce_generator_7);
    add(b, -1);
    add(c, 1);
    solved_goto(b % 10, before_reduce_generator_7);
    add(b, -1);
    add(c, 1);
    solved_goto(b % 10, before_reduce_generator_7);
    add(b, -1);
    add(c, 1);
    solved_goto(b % 10, before_reduce_generator_7);
    add(b, -1);
    add(c, 1);
    goto reduce_by_5;
do_reduce_by_5:
reduce_by_1__6:
    solved_goto(c % 10, reduce_by_10__6);
    add(c, -1);
    add(b, 1);
    goto reduce_by_1__6;
reduce_by_10__6:
    solved_goto(c, after_move_loop__6);
    add(c, -10);
    add(b, 10);
    goto reduce_by_10__6;
after_move_loop__6:
reduce_b_loop__3:
    solved_goto(b, before_reduce_a_loop__3);
    add(b, -5);
    add(c, 1);
    goto reduce_b_loop__3;
before_reduce_a_loop__3:
reduce_by_1__1__3:
    solved_goto(c % 10, reduce_by_10__1__3);
    add(c, -1);
    add(b, 1);
    goto reduce_by_1__1__3;
reduce_by_10__1__3:
    solved_goto(c, after_move_loop__1__3);
    add(c, -10);
    add(b, 10);
    goto reduce_by_10__1__3;
after_move_loop__1__3:
reduce_a_loop__3:
    solved_goto(a, after_reduce_a_loop__3);
    add(a, -1);
    add(c, 5);
    goto reduce_a_loop__3;
after_reduce_a_loop__3:
reduce_by_1__2__3:
    solved_goto(c % 10, reduce_by_10__2__3);
    add(c, -1);
    add(a, 1);
    goto reduce_by_1__2__3;
reduce_by_10__2__3:
    solved_goto(c, after_move_loop__2__3);
    add(c, -10);
    add(a, 10);
    goto reduce_by_10__2__3;
after_move_loop__2__3:
    goto reduce_by_5;
before_reduce_generator_7:
reduce_by_1__7:
    solved_goto(c % 10, reduce_by_10__7);
    add(c, -1);
    add(b, 1);
    goto reduce_by_1__7;
reduce_by_10__7:
    solved_goto(c, after_move_loop__7);
    add(c, -10);
    add(b, 10);
    goto reduce_by_10__7;
after_move_loop__7:
reduce_generator_7:
    add(b, -1);
    solved_goto(b % 10, before_reduce_generator_11);
    add(b, 1);
reduce_b_loop__4:
    solved_goto(b, before_reduce_a_loop__4);
    add(b, -7);
    add(c, 1);
    goto reduce_b_loop__4;
before_reduce_a_loop__4:
reduce_by_1__1__4:
    solved_goto(c % 10, reduce_by_10__1__4);
    add(c, -1);
    add(b, 1);
    goto reduce_by_1__1__4;
reduce_by_10__1__4:
    solved_goto(c, after_move_loop__1__4);
    add(c, -10);
    add(b, 10);
    goto reduce_by_10__1__4;
after_move_loop__1__4:
reduce_a_loop__4:
    solved_goto(a, after_reduce_a_loop__4);
    add(a, -1);
    add(c, 7);
    goto reduce_a_loop__4;
after_reduce_a_loop__4:
reduce_by_1__2__4:
    solved_goto(c % 10, reduce_by_10__2__4);
    add(c, -1);
    add(a, 1);
    goto reduce_by_1__2__4;
reduce_by_10__2__4:
    solved_goto(c, after_move_loop__2__4);
    add(c, -10);
    add(a, 10);
    goto reduce_by_10__2__4;
after_move_loop__2__4:
    goto reduce_generator_7;
reduce_generator_11:
    add(b, -1);
before_reduce_generator_11:
    solved_goto(b, return_);
    add(b, 1);
reduce_b_loop__5:
    solved_goto(b, before_reduce_a_loop__5);
    add(b, -11);
    add(c, 1);
    goto reduce_b_loop__5;
before_reduce_a_loop__5:
reduce_by_1__1__5:
    solved_goto(c % 10, reduce_by_10__1__5);
    add(c, -1);
    add(b, 1);
    goto reduce_by_1__1__5;
reduce_by_10__1__5:
    solved_goto(c, after_move_loop__1__5);
    add(c, -10);
    add(b, 10);
    goto reduce_by_10__1__5;
after_move_loop__1__5:
reduce_a_loop__5:
    solved_goto(a, after_reduce_a_loop__5);
    add(a, -1);
    add(c, 11);
    goto reduce_a_loop__5;
after_reduce_a_loop__5:
reduce_by_1__2__5:
    solved_goto(c % 10, reduce_by_10__2__5);
    add(c, -1);
    add(a, 1);
    goto reduce_by_1__2__5;
reduce_by_10__2__5:
    solved_goto(c, after_move_loop__2__5);
    add(c, -10);
    add(a, 10);
    goto reduce_by_10__2__5;
after_move_loop__2__5:
    goto reduce_generator_11;
move_0_b:
move_const_loop__1:
    solved_goto(b, after_move_const_loop__1);
    add(b, -1);
    goto move_const_loop__1;
after_move_const_loop__1:
    goto return_;
move_0_a:
move_const_loop__2:
    solved_goto(a, after_move_const_loop__2);
    add(a, -1);
    goto move_const_loop__2;
after_move_const_loop__2:
return_:
    return;
}

typedef struct result
{
    char *comp;
    int count;
    int branch;
} result_t;

int main()
{
    char buf[100];
    result_t results[900];
    srand(time(NULL));
    for (int i = 0; i < 30; i++)
    {
        for (int j = 0; j < 30; j++)
        {
            int a = i;
            int b = j;
            int c = 0;
            count = 0;
            branch = 0;
            // WORKS BETTER when A is the larger value compared to B
            multiply(&a, &b, &c);
            snprintf(buf, sizeof(buf), "%d * %d = %d", i, j, a);
            results[i * 30 + j].comp = strdup(buf);
            results[i * 30 + j].count = (int)(count * 10);
            results[i * 30 + j].branch = branch;
            assert(((long long)i * (long long)j) % 30 == a);
            assert(b == 0);
            assert(c == 0);
        }
    }

    for (int i = 1; i < 900; i++)
    {
        result_t key = results[i];
        int j = i - 1;
        while (j >= 0 && results[j].count > key.count)
        {
            results[j + 1] = results[j];
            j--;
        }
        results[j + 1] = key;
    }

    int total = 0;
    int total2 = 0;
    for (int i = 0; i < 900; i++)
    {
        printf("%s: %d moves; %d branches\n", results[i].comp, results[i].count, results[i].branch);
        total += results[i].count;
        total2 += results[i].branch;
    }
    // average count per multiplication
    printf("\nAverage: %f moves; %f branches\n", (double)total / 900, (double)total2 / 900);
    return 0;
}
