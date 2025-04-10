#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <time.h>
#include <assert.h>

int branch = 0;
float count = 0;

void multiply(int *a, int *b, int *c)
{
    branch++;
    if (*a == 0)
    {
        goto move_0_b;
    }
    branch++;
    if (*b == 0)
    {
        goto move_0_a;
    }
reduce_by_2:
    branch++;
    if (*b % 10 == 0)
    {
        goto do_reduce_by_2;
    }
    *b -= 1;
    *c += 1;
    count += 1.5;
    branch++;
    if (*b % 10 == 0)
    {
        goto before_reduce_by_3;
    }
    *b -= 1;
    *c += 1;
    count += 1.5;
    goto reduce_by_2;
do_reduce_by_2:
move3__reduce_by_1__1:
    branch++;
    if (*c % 3 == 0)
    {
        goto move3__reduce_by_3__1;
    }
    *c -= 1;
    *b += 1;
    count += 1.5;
    goto move3__reduce_by_1__1;
move3__reduce_by_3__1:
    branch++;
    if (*c == 0)
    {
        goto move3__after_move_loop__1;
    }
    *c -= 3;
    *b += 3;
    count += 1.5;
    goto move3__reduce_by_3__1;
move3__after_move_loop__1:
reduce_problem__reduce_b_loop__1:
    branch++;
    if (*b == 0)
    {
        goto reduce_problem__before_reduce_a_loop__1;
    }
    *b -= 2;
    *b = (*b % 30 + 30) % 30;
    *c += 1;
    count += 1.5;
    goto reduce_problem__reduce_b_loop__1;
reduce_problem__before_reduce_a_loop__1:
reduce_problem__move3__reduce_by_1__1__1:
    branch++;
    if (*c % 3 == 0)
    {
        goto reduce_problem__move3__reduce_by_3__1__1;
    }
    *c -= 1;
    *b += 1;
    count += 1.5;
    goto reduce_problem__move3__reduce_by_1__1__1;
reduce_problem__move3__reduce_by_3__1__1:
    branch++;
    if (*c == 0)
    {
        goto reduce_problem__move3__after_move_loop__1__1;
    }
    *c -= 3;
    *b += 3;
    count += 1.5;
    goto reduce_problem__move3__reduce_by_3__1__1;
reduce_problem__move3__after_move_loop__1__1:
reduce_problem__reduce_a_loop__1:
    branch++;
    if (*a == 0)
    {
        goto reduce_problem__after_reduce_a_loop__1;
    }
    *a -= 1;
    *c = (*c + 2) % 30;
    count += 1.5;
    goto reduce_problem__reduce_a_loop__1;
reduce_problem__after_reduce_a_loop__1:
reduce_problem__move10__reduce_by_1__1__1:
    branch++;
    if (*c % 10 == 0)
    {
        goto reduce_problem__move10__reduce_by_10__1__1;
    }
    *c -= 1;
    *a += 1;
    count += 1.5;
    goto reduce_problem__move10__reduce_by_1__1__1;
reduce_problem__move10__reduce_by_10__1__1:
    branch++;
    if (*c == 0)
    {
        goto reduce_problem__move10__after_move_loop__1__1;
    }
    *c -= 10;
    *a += 10;
    count += 1.5;
    goto reduce_problem__move10__reduce_by_10__1__1;
reduce_problem__move10__after_move_loop__1__1:
    goto reduce_by_2;
do_reduce_by_3:
raw_move__raw_move_loop__1:
    branch++;
    if (*c == 0)
    {
        goto raw_move__after_raw_move_loop__1;
    }
    *b += 1;
    *c -= 1;
    count += 1.5;
    goto raw_move__raw_move_loop__1;
raw_move__after_raw_move_loop__1:
reduce_problem__reduce_b_loop__2:
    branch++;
    if (*b == 0)
    {
        goto reduce_problem__before_reduce_a_loop__2;
    }
    *b -= 3;
    *b = (*b % 30 + 30) % 30;
    *c += 1;
    count += 1.5;
    goto reduce_problem__reduce_b_loop__2;
reduce_problem__before_reduce_a_loop__2:
reduce_problem__move3__reduce_by_1__1__2:
    branch++;
    if (*c % 3 == 0)
    {
        goto reduce_problem__move3__reduce_by_3__1__2;
    }
    *c -= 1;
    *b += 1;
    count += 1.5;
    goto reduce_problem__move3__reduce_by_1__1__2;
reduce_problem__move3__reduce_by_3__1__2:
    branch++;
    if (*c == 0)
    {
        goto reduce_problem__move3__after_move_loop__1__2;
    }
    *c -= 3;
    *b += 3;
    count += 1.5;
    goto reduce_problem__move3__reduce_by_3__1__2;
reduce_problem__move3__after_move_loop__1__2:
reduce_problem__reduce_a_loop__2:
    branch++;
    if (*a == 0)
    {
        goto reduce_problem__after_reduce_a_loop__2;
    }
    *a -= 1;
    *c = (*c + 3) % 30;
    count += 1.5;
    goto reduce_problem__reduce_a_loop__2;
reduce_problem__after_reduce_a_loop__2:
reduce_problem__move10__reduce_by_1__1__2:
    branch++;
    if (*c % 10 == 0)
    {
        goto reduce_problem__move10__reduce_by_10__1__2;
    }
    *c -= 1;
    *a += 1;
    count += 1.5;
    goto reduce_problem__move10__reduce_by_1__1__2;
reduce_problem__move10__reduce_by_10__1__2:
    branch++;
    if (*c == 0)
    {
        goto reduce_problem__move10__after_move_loop__1__2;
    }
    *c -= 10;
    *a += 10;
    count += 1.5;
    goto reduce_problem__move10__reduce_by_10__1__2;
reduce_problem__move10__after_move_loop__1__2:
    goto reduce_by_3;
before_reduce_by_3:
move3__reduce_by_1__2:
    branch++;
    if (*c % 3 == 0)
    {
        goto move3__reduce_by_3__2;
    }
    *c -= 1;
    *b += 1;
    count += 1.5;
    goto move3__reduce_by_1__2;
move3__reduce_by_3__2:
    branch++;
    if (*c == 0)
    {
        goto move3__after_move_loop__2;
    }
    *c -= 3;
    *b += 3;
    count += 1.5;
    goto move3__reduce_by_3__2;
move3__after_move_loop__2:
reduce_by_3:
    branch++;
    if (*b % 3 == 0)
    {
        goto do_reduce_by_3;
    }
reduce_by_5:
    branch++;
    if (*b % 10 == 0)
    {
        goto do_reduce_by_5;
    }
    *b -= 1;
    *c += 1;
    count += 1.5;
    branch++;
    if (*b % 10 == 0)
    {
        goto before_reduce_generator_7;
    }
    *b -= 1;
    *c += 1;
    count += 1.5;
    branch++;
    if (*b % 10 == 0)
    {
        goto before_reduce_generator_7;
    }
    *b -= 1;
    *c += 1;
    count += 1.5;
    branch++;
    if (*b % 10 == 0)
    {
        goto before_reduce_generator_7;
    }
    *b -= 1;
    *c += 1;
    count += 1.5;
    branch++;
    if (*b % 10 == 0)
    {
        goto before_reduce_generator_7;
    }
    *b -= 1;
    *c += 1;
    count += 1.5;
    goto reduce_by_5;
do_reduce_by_5:
move3__reduce_by_1__3:
    branch++;
    if (*c % 3 == 0)
    {
        goto move3__reduce_by_3__3;
    }
    *c -= 1;
    *b += 1;
    count += 1.5;
    goto move3__reduce_by_1__3;
move3__reduce_by_3__3:
    branch++;
    if (*c == 0)
    {
        goto move3__after_move_loop__3;
    }
    *c -= 3;
    *b += 3;
    count += 1.5;
    goto move3__reduce_by_3__3;
move3__after_move_loop__3:
reduce_problem__reduce_b_loop__3:
    branch++;
    if (*b == 0)
    {
        goto reduce_problem__before_reduce_a_loop__3;
    }
    *b -= 5;
    *b = (*b % 30 + 30) % 30;
    *c += 1;
    count += 1.5;
    goto reduce_problem__reduce_b_loop__3;
reduce_problem__before_reduce_a_loop__3:
reduce_problem__move3__reduce_by_1__1__3:
    branch++;
    if (*c % 3 == 0)
    {
        goto reduce_problem__move3__reduce_by_3__1__3;
    }
    *c -= 1;
    *b += 1;
    count += 1.5;
    goto reduce_problem__move3__reduce_by_1__1__3;
reduce_problem__move3__reduce_by_3__1__3:
    branch++;
    if (*c == 0)
    {
        goto reduce_problem__move3__after_move_loop__1__3;
    }
    *c -= 3;
    *b += 3;
    count += 1.5;
    goto reduce_problem__move3__reduce_by_3__1__3;
reduce_problem__move3__after_move_loop__1__3:
reduce_problem__reduce_a_loop__3:
    branch++;
    if (*a == 0)
    {
        goto reduce_problem__after_reduce_a_loop__3;
    }
    *a -= 1;
    *c = (*c + 5) % 30;
    count += 1.5;
    goto reduce_problem__reduce_a_loop__3;
reduce_problem__after_reduce_a_loop__3:
reduce_problem__move10__reduce_by_1__1__3:
    branch++;
    if (*c % 10 == 0)
    {
        goto reduce_problem__move10__reduce_by_10__1__3;
    }
    *c -= 1;
    *a += 1;
    count += 1.5;
    goto reduce_problem__move10__reduce_by_1__1__3;
reduce_problem__move10__reduce_by_10__1__3:
    branch++;
    if (*c == 0)
    {
        goto reduce_problem__move10__after_move_loop__1__3;
    }
    *c -= 10;
    *a += 10;
    count += 1.5;
    goto reduce_problem__move10__reduce_by_10__1__3;
reduce_problem__move10__after_move_loop__1__3:
    goto reduce_by_5;
before_reduce_generator_7:
move3__reduce_by_1__4:
    branch++;
    if (*c % 3 == 0)
    {
        goto move3__reduce_by_3__4;
    }
    *c -= 1;
    *b += 1;
    count += 1.5;
    goto move3__reduce_by_1__4;
move3__reduce_by_3__4:
    branch++;
    if (*c == 0)
    {
        goto move3__after_move_loop__4;
    }
    *c -= 3;
    *b += 3;
    count += 1.5;
    goto move3__reduce_by_3__4;
move3__after_move_loop__4:
reduce_generator_7:
    *b -= 1;
    count++;
    branch++;
    if (*b % 10 == 0)
    {
        goto before_reduce_generator_11;
    }
    *b += 1;
    count++;
reduce_problem__reduce_b_loop__4:
    branch++;
    if (*b == 0)
    {
        goto reduce_problem__before_reduce_a_loop__4;
    }
    *b -= 7;
    *b = (*b % 30 + 30) % 30;
    *c += 1;
    count += 1.5;
    goto reduce_problem__reduce_b_loop__4;
reduce_problem__before_reduce_a_loop__4:
reduce_problem__move3__reduce_by_1__1__4:
    branch++;
    if (*c % 3 == 0)
    {
        goto reduce_problem__move3__reduce_by_3__1__4;
    }
    *c -= 1;
    *b += 1;
    count += 1.5;
    goto reduce_problem__move3__reduce_by_1__1__4;
reduce_problem__move3__reduce_by_3__1__4:
    branch++;
    if (*c == 0)
    {
        goto reduce_problem__move3__after_move_loop__1__4;
    }
    *c -= 3;
    *b += 3;
    count += 1.5;
    goto reduce_problem__move3__reduce_by_3__1__4;
reduce_problem__move3__after_move_loop__1__4:
reduce_problem__reduce_a_loop__4:
    branch++;
    if (*a == 0)
    {
        goto reduce_problem__after_reduce_a_loop__4;
    }
    *a -= 1;
    *c = (*c + 7) % 30;
    count += 1.5;
    goto reduce_problem__reduce_a_loop__4;
reduce_problem__after_reduce_a_loop__4:
reduce_problem__move10__reduce_by_1__1__4:
    branch++;
    if (*c % 10 == 0)
    {
        goto reduce_problem__move10__reduce_by_10__1__4;
    }
    *c -= 1;
    *a += 1;
    count += 1.5;
    goto reduce_problem__move10__reduce_by_1__1__4;
reduce_problem__move10__reduce_by_10__1__4:
    branch++;
    if (*c == 0)
    {
        goto reduce_problem__move10__after_move_loop__1__4;
    }
    *c -= 10;
    *a += 10;
    count += 1.5;
    goto reduce_problem__move10__reduce_by_10__1__4;
reduce_problem__move10__after_move_loop__1__4:
    goto reduce_generator_7;
reduce_generator_11:
    *b -= 1;
    count++;
before_reduce_generator_11:
    branch++;
    if (*b == 0)
    {
        goto return_;
    }
    *b += 1;
    count++;
reduce_problem__reduce_b_loop__5:
    branch++;
    if (*b == 0)
    {
        goto reduce_problem__before_reduce_a_loop__5;
    }
    *b -= 11;
    *b = (*b % 30 + 30) % 30;
    *c += 1;
    count += 1.5;
    goto reduce_problem__reduce_b_loop__5;
reduce_problem__before_reduce_a_loop__5:
reduce_problem__move3__reduce_by_1__1__5:
    branch++;
    if (*c % 3 == 0)
    {
        goto reduce_problem__move3__reduce_by_3__1__5;
    }
    *c -= 1;
    *b += 1;
    count += 1.5;
    goto reduce_problem__move3__reduce_by_1__1__5;
reduce_problem__move3__reduce_by_3__1__5:
    branch++;
    if (*c == 0)
    {
        goto reduce_problem__move3__after_move_loop__1__5;
    }
    *c -= 3;
    *b += 3;
    count += 1.5;
    goto reduce_problem__move3__reduce_by_3__1__5;
reduce_problem__move3__after_move_loop__1__5:
reduce_problem__reduce_a_loop__5:
    branch++;
    if (*a == 0)
    {
        goto reduce_problem__after_reduce_a_loop__5;
    }
    *a -= 1;
    *c = (*c + 11) % 30;
    count += 1.5;
    goto reduce_problem__reduce_a_loop__5;
reduce_problem__after_reduce_a_loop__5:
reduce_problem__move10__reduce_by_1__1__5:
    branch++;
    if (*c % 10 == 0)
    {
        goto reduce_problem__move10__reduce_by_10__1__5;
    }
    *c -= 1;
    *a += 1;
    count += 1.5;
    goto reduce_problem__move10__reduce_by_1__1__5;
reduce_problem__move10__reduce_by_10__1__5:
    branch++;
    if (*c == 0)
    {
        goto reduce_problem__move10__after_move_loop__1__5;
    }
    *c -= 10;
    *a += 10;
    count += 1.5;
    goto reduce_problem__move10__reduce_by_10__1__5;
reduce_problem__move10__after_move_loop__1__5:
    goto reduce_generator_11;
move_0_b:
move_const__move_const_loop__1:
    branch++;
    if (*b == 0)
    {
        goto move_const__after_move_const_loop__1;
    }
    *b -= 1;
    count++;
    goto move_const__move_const_loop__1;
move_const__after_move_const_loop__1:
    goto return_;
move_0_a:
move_const__move_const_loop__2:
    branch++;
    if (*a == 0)
    {
        goto move_const__after_move_const_loop__2;
    }
    *a -= 1;
    count++;
    goto move_const__move_const_loop__2;
move_const__after_move_const_loop__2:
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
