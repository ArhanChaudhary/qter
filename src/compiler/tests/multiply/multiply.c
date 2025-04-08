#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <time.h>
#include <assert.h>

int branch = 0;
int count = 0;

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
    branch++;
    if (n != 0)
        count++;
}

void move(int *from, int *to)
{
// TODO: rokicki exponentiation
move_loop:
    branch++;
    if (*from == 0)
    {
        return;
    }
    *to += 1;
    *from -= 1;
    count += 2;
    goto move_loop;
}

void reduce_problem(int *a, int *b, int *c, const int k)
{
    *b -= 1;
    count++;
    branch++;
    if (*b == 0)
    {
        *b += 1;
        count++;
        return;
    }
    *b += 1;
    count++;
reduce_b_loop:
    branch++;
    if (*b == 0)
    {
        move(c, b);
        goto reduce_a_loop;
    }
    *b -= k;
    *b = (*b % 30 + 30) % 30;
    *c += 1;
    count += 2;
    goto reduce_b_loop;
reduce_a_loop:
    branch++;
    if (*a == 0)
    {
        move(c, a);
        return;
    }
    *a -= 1;
    *c = (*c + k) % 30;
    count += 2;
    goto reduce_a_loop;
}

void multiply(int *a, int *b, int *c)
{
    move_const(0, c);
    branch++;
    if (*a == 0)
    {
        move_const(0, b);
        return;
    }
    branch++;
    if (*b == 0)
    {
        move_const(0, a);
        return;
    }
reduce_by_2:
    branch++;
    if (*b % 10 == 0)
    {
        move(c, b);
        reduce_problem(a, b, c, 2);
        goto reduce_by_2;
    }
    *b -= 1;
    *c += 1;
    count += 2;
    branch++;
    if (*b % 10 == 0)
    {
        move(c, b);
        goto reduce_by_3;
    }
    *b -= 1;
    *c += 1;
    count += 2;
    goto reduce_by_2;
reduce_by_3:
    branch++;
    if (*b % 3 == 0)
    {
        move(c, b);
        reduce_problem(a, b, c, 3);
        goto reduce_by_3;
    }
reduce_by_5:
    branch++;
    if (*b % 10 == 0)
    {
        move(c, b);
        reduce_problem(a, b, c, 5);
        goto reduce_by_5;
    }
    *b -= 1;
    *c += 1;
    count += 2;
    branch++;
    if (*b % 10 == 0)
    {
        goto before_reduce_generator_7;
    }
    *b -= 1;
    *c += 1;
    count += 2;
    branch++;
    if (*b % 10 == 0)
    {
        goto before_reduce_generator_7;
    }
    *b -= 1;
    *c += 1;
    count += 2;
    branch++;
    if (*b % 10 == 0)
    {
        goto before_reduce_generator_7;
    }
    *b -= 1;
    *c += 1;
    count += 2;
    branch++;
    if (*b % 10 == 0)
    {
        goto before_reduce_generator_7;
    }
    *b -= 1;
    *c += 1;
    count += 2;
    goto reduce_by_5;
before_reduce_generator_7:
    move(c, b);
reduce_generator_7:
    *b -= 1;
    branch++;
    if (*b % 10 == 0)
    {
        *b += 1;
        goto reduce_generator_11;
    }
    *b += 1;
    reduce_problem(a, b, c, 7);
    goto reduce_generator_7;
reduce_generator_11:
    *b -= 1;
    branch++;
    if (*b == 0)
    {
        return;
    }
    *b += 1;
    reduce_problem(a, b, c, 11);
    goto reduce_generator_11;
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
            multiply(&a, &b, &c);
            snprintf(buf, sizeof(buf), "%d * %d = %d", i, j, a);
            results[i * 30 + j].comp = strdup(buf);
            results[i * 30 + j].count = count;
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
    for (int i = 0; i < 900; i++)
    {
        printf("%s: %d additions; %d branches\n", results[i].comp, results[i].count, results[i].branch);
        total += results[i].count;
    }
    // average count per multiplication
    printf("Average: %f\n", (double)total / 900);
    return 0;
}
