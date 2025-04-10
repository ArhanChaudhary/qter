#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <time.h>
#include <assert.h>

int branch = 0;
float count = 0;

void move_const(int n, int *to)
{
move_const_loop:
    branch++;
    if (*to == 0)
    {
        goto after_move_const_loop;
    }
    *to -= 1;
    count++;
    goto move_const_loop;
after_move_const_loop:
    *to += n;
    if (n != 0)
        count++;
}

void move(int *from, int *to)
{
reduce_by_1:
    branch++;
    if (*from % 10 == 0)
    {
        goto reduce_by_10;
    }
    *from -= 1;
    *to += 1;
    count += 1.5;
    goto reduce_by_1;
reduce_by_10:
    branch++;
    if (*from == 0)
    {
        goto after_move_loop;
    }
    *from -= 10;
    *to += 10;
    count += 1.5;
    goto reduce_by_10;
after_move_loop:
}

void reduce_problem(int *a, int *b, int *c, const int k)
{
reduce_b_loop:
    branch++;
    if (*b == 0)
    {
        goto before_reduce_a_loop;
    }
    *b -= k;
    *b = (*b % 30 + 30) % 30;
    *c += 1;
    count += 1.5;
    goto reduce_b_loop;
before_reduce_a_loop:
    move(c, b);
reduce_a_loop:
    branch++;
    if (*a == 0)
    {
        goto after_reduce_a_loop;
    }
    *a -= 1;
    *c = (*c + k) % 30;
    count += 1.5;
    goto reduce_a_loop;
after_reduce_a_loop:
    move(c, a);
}

void multiply(int *a, int *b, int *c)
{
    // move_const(0, c);
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
    move(c, b);
    reduce_problem(a, b, c, 2);
    goto reduce_by_2;
do_reduce_by_3:
    move(c, b);
    reduce_problem(a, b, c, 3);
    goto reduce_by_3;
before_reduce_by_3:
    move(c, b);
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
    move(c, b);
    reduce_problem(a, b, c, 5);
    goto reduce_by_5;
before_reduce_generator_7:
    move(c, b);
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
    reduce_problem(a, b, c, 7);
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
    reduce_problem(a, b, c, 11);
    goto reduce_generator_11;
move_0_b:
    move_const(0, b);
    goto return_;
move_0_a:
    move_const(0, a);
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
