#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <assert.h>

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
        *a = 0;
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
        goto guard_1;
    }
    goto after_guard_1;
guard_1:
    if (*a == 0)
    {
        return;
    }
    *a -= 1;
    goto guard_1;
after_guard_1:
    if (*a == 0)
    {
        return;
    }
nullify_c:
    if (*c == 0)
    {
        goto reduce_by_2;
    }
    *c -= 1;
    goto nullify_c;
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

int main()
{
    srand(time(NULL));
    for (int i = 0; i < 90; i++)
    {
        for (int j = 0; j < 90; j++)
        {
            int a = i;
            int b = j;
            int c = rand() % 90;
            printf("%d * %d = ", i, j);
            multiply(&a, &b, &c);
            printf("%d\n", a);

            assert(((long long)i * (long long)j) % 90 == a);
        }
    }
    return 0;
}
