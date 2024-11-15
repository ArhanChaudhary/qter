#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <time.h>
#include <assert.h>

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
    *c += 1;
    if (*b == 0)
    {
        goto reduce_by_3;
    }
    *b -= 1;
    *c += 1;
    goto reduce_by_2;
reduce_by_3:
    if (*c == 0)
    {
        move(b, c);
        reduce_problem(a, c, b, 3);
        goto reduce_by_3;
    }
    *c -= 1;
    *b += 1;
    if (*c == 0)
    {
        goto reduce_by_5;
    }
    *c -= 1;
    *b += 1;
    if (*c == 0)
    {
        goto reduce_by_5;
    }
    *c -= 1;
    *b += 1;
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
        goto reduce_until_1;
    }
    *b -= 1;
    *c += 1;
    if (*b == 0)
    {
        goto reduce_until_1;
    }
    *b -= 1;
    *c += 1;
    if (*b == 0)
    {
        goto reduce_until_1;
    }
    *b -= 1;
    *c += 1;
    if (*b == 0)
    {
        goto reduce_until_1;
    }
    *b -= 1;
    *c += 1;
    goto reduce_by_5;
reduce_until_1:
    *c -= 1;
    if (*c == 0)
    {
        return;
    }
    *c += 1;
    reduce_problem(a, c, b, 7);
    reduce_problem(a, c, b, 11);
    goto reduce_until_1;
}

int main()
{
    for (int i = 0; i < 90; i++)
    {
        for (int j = 0; j < 90; j++)
        {
            int a = i;
            int b = j;
            int c = 0;
            multiply(&a, &b, &c);
            printf("%d * %d = %d", i, j, a);
            assert(((long long)i * (long long)j) % 90 == a);
            assert(b == 0);
            assert(c == 0);
        }
    }
}
