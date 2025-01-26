#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <time.h>
#include <assert.h>

int count = 0;

void move_const(int n, int *to)
{
move_const_loop:
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
move_loop:
    if (*from == 0)
    {
        return;
    }
    *to += 1;
    count++;
    *from -= 1;
    count++;
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
    count++;
    if (*b == 0)
    {
        *b += 1;
        count++;
        return;
    }
    *b += 1;
    count++;
reduce_b_loop:
    if (*b == 0)
    {
        move(c, b);
        goto reduce_a_loop;
    }
    *b -= k;
    *b = (*b % 30 + 30) % 30;
    count += 2;
    *c += 1;
    count++;
    goto reduce_b_loop;
reduce_a_loop:
    if (*a == 0)
    {
        move(c, a);
        return;
    }
    *a -= 1;
    count++;
    *c = (*c + k) % 30;
    count += 2;
    goto reduce_a_loop;
}

void multiply(int *a, int *b, int *c)
{
    move_const(0, c);
    if (*a == 0)
    {
        move_const(0, b);
        return;
    }
    if (*b == 0)
    {
        move_const(0, a);
        return;
    }
reduce_by_2:
    if (*b == 0)
    {
        move(c, b);
        reduce_problem(a, b, c, 2);
        goto reduce_by_2;
    }
    *b -= 1;
    count++;
    *c += 1;
    count++;
    if (*b == 0)
    {
        goto reduce_by_3;
    }
    *b -= 1;
    count++;
    *c += 1;
    count++;
    goto reduce_by_2;
reduce_by_3:
    if (*c == 0)
    {
        move(b, c);
        reduce_problem(a, c, b, 3);
        goto reduce_by_3;
    }
    *c -= 1;
    count++;
    *b += 1;
    count++;
    if (*c == 0)
    {
        goto reduce_by_5;
    }
    *c -= 1;
    count++;
    *b += 1;
    count++;
    if (*c == 0)
    {
        goto reduce_by_5;
    }
    *c -= 1;
    count++;
    *b += 1;
    count++;
    goto reduce_by_3;
reduce_by_5:
    if (*b == 0)
    {
        move(c, b);
        reduce_problem(a, b, c, 5);
        goto reduce_by_5;
    }
    *b -= 1;
    count++;
    *c += 1;
    count++;
    if (*b == 0)
    {
        goto reduce_until_1;
    }
    *b -= 1;
    count++;
    *c += 1;
    count++;
    if (*b == 0)
    {
        goto reduce_until_1;
    }
    *b -= 1;
    count++;
    *c += 1;
    count++;
    if (*b == 0)
    {
        goto reduce_until_1;
    }
    *b -= 1;
    count++;
    *c += 1;
    count++;
    if (*b == 0)
    {
        goto reduce_until_1;
    }
    *b -= 1;
    count++;
    *c += 1;
    count++;
    goto reduce_by_5;
reduce_until_1:
    *c -= 1;
    count++;
    if (*c == 0)
    {
        return;
    }
    *c += 1;
    count++;
    reduce_problem(a, c, b, 7);
    reduce_problem(a, c, b, 11);
    goto reduce_until_1;
}

typedef struct result {
    char *comp;
    int count;
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
            multiply(&a, &b, &c);
            snprintf(buf, sizeof(buf), "%d * %d = %d", i, j, a);
            results[i * 30 + j].comp = strdup(buf);
            results[i * 30 + j].count = count;
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
        printf("%s: %d additions\n", results[i].comp, results[i].count);
        total += results[i].count;
    }
    // average count per multiplication
    printf("Average: %f\n", (double)total / 900);
    return 0;
}
