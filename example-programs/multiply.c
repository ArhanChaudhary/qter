#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <time.h>
#include <assert.h>

int count = 0;
int count2 = 0;

void move_const(int n, int *to)
{
move_const_loop:
    if (*to == 0)
    {
        goto after_move_const_loop;
    }
    *to -= 1;
    goto move_const_loop;
after_move_const_loop:
    if (n != 0) {
    }
    *to += n;
}

void move(int *from, int *to)
{
move_loop:
    if (*from == 0)
    {
        return;
    }
    *to += 1;
    *from -= 1;
    goto move_loop;
}

void reduce_problem(int *a, int *b, int *c, const int k)
{
    if (*b == 0)
    {
        move_const(0, a);
        return;
    }
    *b -= 1;
    if (*b == 0)
    {
        *b += 1;
        return;
    }
    *b += 1;
reduce_b_loop:
    if (*b == 0)
    {
        move(c, b);
        goto reduce_a_loop;
    }
    *b = (*b - k) % 90;
    *c += 1;
    goto reduce_b_loop;
reduce_a_loop:
    if (*a == 0)
    {
        move(c, a);
        return;
    }
    *a -= 1;
    *c = (*c + k) % 90;
    goto reduce_a_loop;
}

void multiply(int *a, int *b, int *c)
{
    if (*b == 0)
    {
        move_const(0, a);
        return;
    }
    if (*a == 0)
    {
        return;
    }
    move_const(0, c);
reduce_by_2:
    if (*b == 0)
    {
        move(c, b);
        reduce_problem(a, b, c, 2);
        goto reduce_by_2;
    }
    *b -= 1;
    *c += 1;
    if (*b == 0)
    {
        move(c, b);
        goto reduce_by_3;
    }
    *b -= 1;
    *c += 1;
    goto reduce_by_2;
reduce_by_3:
    if (*b == 0)
    {
        move(c, b);
        reduce_problem(a, b, c, 3);
        goto reduce_by_3;
    }
    *b -= 1;
    *c += 1;
    if (*b == 0)
    {
        move(c, b);
        goto reduce_by_5;
    }
    *b -= 1;
    *c += 1;
    if (*b == 0)
    {
        move(c, b);
        goto reduce_by_5;
    }
    *b -= 1;
    *c += 1;
    goto reduce_by_3;
reduce_by_5:
    if (*b == 0)
    {
        move(c, b);
        reduce_problem(a, b, c, 5);
        goto reduce_by_5;
    }
    *b -= 1;
    *c += 1;
    if (*b == 0)
    {
        move(c, b);
        goto reduce_until_1;
    }
    *b -= 1;
    *c += 1;
    if (*b == 0)
    {
        move(c, b);
        goto reduce_until_1;
    }
    *b -= 1;
    *c += 1;
    if (*b == 0)
    {
        move(c, b);
        goto reduce_until_1;
    }
    *b -= 1;
    *c += 1;
    if (*b == 0)
    {
        move(c, b);
        goto reduce_until_1;
    }
    *b -= 1;
    *c += 1;
    goto reduce_by_5;
reduce_until_1:
    *b -= 1;
    if (*b == 0)
    {
        return;
    }
    *b += 1;
    reduce_problem(a, b, c, 7);
    reduce_problem(a, b, c, 11);
    goto reduce_until_1;
}

typedef struct result {
    char *comp;
    int count;
    int count2;
} result_t;

int main()
{
    result_t results[8100];
    char buf[100];
    srand(time(NULL));
    for (int i = 0; i < 90; i++)
    {
        for (int j = 0; j < 90; j++)
        {
            int a = i;
            int b = j;
            int c = rand() % 90;
            count = 0;
            count2 = 0;
            multiply(&a, &b, &c);
            snprintf(buf, sizeof(buf), "%d * %d = %d", i, j, a);
            results[i * 90 + j].comp = strdup(buf);
            results[i * 90 + j].count = count;
            results[i * 90 + j].count2 = count2;
            assert(((long long)i * (long long)j) % 90 == a);
        }
    }

    // insertion sort by count
    for (int i = 1; i < 8100; i++)
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
    for (int i = 0; i < 8100; i++)
    {
        printf("%s: %d additions, %d comparisons\n", results[i].comp, results[i].count, results[i].count2);
    }
    return 0;
}
