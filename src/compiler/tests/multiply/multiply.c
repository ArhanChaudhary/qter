/**
 * A C stub for a QAT multiplication program. The architecture of the program is
 * assumed to be 30/30/30, not working otherwise.
 *
 * Input:
 * - Argument 1: First number (0-29)
 * - Argument 2: Second number (0-29)
 * - Argument 3: 0
 *
 * Output:
 * - Argument 1: Result of multiplication modulo 30
 *
 * Caveats: The program is faster when the first argument is larger than the
 * second.
 */

#include <stdio.h>
#include <assert.h>

#define solved_goto(b, label) \
    if (*b == 0)              \
    {                         \
        goto label;           \
    }

#define add(a, n)                 \
    {                             \
        *a += n;                  \
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
    solved_goto(c % 3, move3__reduce_by_3__1);
    add(c, -1);
    add(b, 1);
    goto do_reduce_by_2;
move3__reduce_by_3__1:
    solved_goto(c, move3__after_move_loop__1);
    add(c, -3);
    add(b, 3);
    goto move3__reduce_by_3__1;
move3__after_move_loop__1:
    solved_goto(b, reduce_problem__before_reduce_a_loop__1);
    add(b, -2);
    add(c, 1);
    goto move3__after_move_loop__1;
reduce_problem__before_reduce_a_loop__1:
    solved_goto(c % 3, reduce_problem__move3__reduce_by_3__1__1);
    add(c, -1);
    add(b, 1);
    goto reduce_problem__before_reduce_a_loop__1;
reduce_problem__move3__reduce_by_3__1__1:
    solved_goto(c, reduce_problem__move3__after_move_loop__1__1);
    add(c, -3);
    add(b, 3);
    goto reduce_problem__move3__reduce_by_3__1__1;
reduce_problem__move3__after_move_loop__1__1:
    solved_goto(a, reduce_problem__after_reduce_a_loop__1);
    add(a, -1);
    add(c, 2);
    goto reduce_problem__move3__after_move_loop__1__1;
reduce_problem__after_reduce_a_loop__1:
    solved_goto(c % 10, reduce_problem__move10__reduce_by_10__1__1);
    add(c, -1);
    add(a, 1);
    goto reduce_problem__after_reduce_a_loop__1;
reduce_problem__move10__reduce_by_10__1__1:
    solved_goto(c, reduce_by_2);
    add(c, -10);
    add(a, 10);
    goto reduce_problem__move10__reduce_by_10__1__1;
do_reduce_by_3:
    solved_goto(c, raw_move__after_raw_move_loop__1);
    add(b, 1);
    add(c, -1);
    goto do_reduce_by_3;
raw_move__after_raw_move_loop__1:
    solved_goto(b, reduce_problem__before_reduce_a_loop__2);
    add(b, -3);
    add(c, 1);
    goto raw_move__after_raw_move_loop__1;
reduce_problem__before_reduce_a_loop__2:
    solved_goto(c % 3, reduce_problem__move3__reduce_by_3__1__2);
    add(c, -1);
    add(b, 1);
    goto reduce_problem__before_reduce_a_loop__2;
reduce_problem__move3__reduce_by_3__1__2:
    solved_goto(c, reduce_problem__move3__after_move_loop__1__2);
    add(c, -3);
    add(b, 3);
    goto reduce_problem__move3__reduce_by_3__1__2;
reduce_problem__move3__after_move_loop__1__2:
    solved_goto(a, reduce_problem__after_reduce_a_loop__2);
    add(a, -1);
    add(c, 3);
    goto reduce_problem__move3__after_move_loop__1__2;
reduce_problem__after_reduce_a_loop__2:
    solved_goto(c % 10, reduce_problem__move10__reduce_by_10__1__2);
    add(c, -1);
    add(a, 1);
    goto reduce_problem__after_reduce_a_loop__2;
reduce_problem__move10__reduce_by_10__1__2:
    solved_goto(c, move3__after_move_loop__2);
    add(c, -10);
    add(a, 10);
    goto reduce_problem__move10__reduce_by_10__1__2;
before_reduce_by_3:
    solved_goto(c % 3, move3__reduce_by_3__2);
    add(c, -1);
    add(b, 1);
    goto before_reduce_by_3;
move3__reduce_by_3__2:
    solved_goto(c, move3__after_move_loop__2);
    add(c, -3);
    add(b, 3);
    goto move3__reduce_by_3__2;
move3__after_move_loop__2:
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
    solved_goto(c % 3, move3__reduce_by_3__3);
    add(c, -1);
    add(b, 1);
    goto do_reduce_by_5;
move3__reduce_by_3__3:
    solved_goto(c, move3__after_move_loop__3);
    add(c, -3);
    add(b, 3);
    goto move3__reduce_by_3__3;
move3__after_move_loop__3:
    solved_goto(b, reduce_problem__before_reduce_a_loop__3);
    add(b, -5);
    add(c, 1);
    goto move3__after_move_loop__3;
reduce_problem__before_reduce_a_loop__3:
    solved_goto(c % 3, reduce_problem__move3__reduce_by_3__1__3);
    add(c, -1);
    add(b, 1);
    goto reduce_problem__before_reduce_a_loop__3;
reduce_problem__move3__reduce_by_3__1__3:
    solved_goto(c, reduce_problem__move3__after_move_loop__1__3);
    add(c, -3);
    add(b, 3);
    goto reduce_problem__move3__reduce_by_3__1__3;
reduce_problem__move3__after_move_loop__1__3:
    solved_goto(a, reduce_problem__after_reduce_a_loop__3);
    add(a, -1);
    add(c, 5);
    goto reduce_problem__move3__after_move_loop__1__3;
reduce_problem__after_reduce_a_loop__3:
    solved_goto(c % 10, reduce_problem__move10__reduce_by_10__1__3);
    add(c, -1);
    add(a, 1);
    goto reduce_problem__after_reduce_a_loop__3;
reduce_problem__move10__reduce_by_10__1__3:
    solved_goto(c, reduce_by_5);
    add(c, -10);
    add(a, 10);
    goto reduce_problem__move10__reduce_by_10__1__3;
before_reduce_generator_7:
    solved_goto(c % 3, move3__reduce_by_3__4);
    add(c, -1);
    add(b, 1);
    goto before_reduce_generator_7;
move3__reduce_by_3__4:
    solved_goto(c, move3__after_move_loop__4);
    add(c, -3);
    add(b, 3);
    goto move3__reduce_by_3__4;
move3__after_move_loop__4:
    add(b, -1);
    solved_goto(b % 10, before_reduce_generator_11);
    add(b, 1);
reduce_problem__reduce_b_loop__4:
    solved_goto(b, reduce_problem__before_reduce_a_loop__4);
    add(b, -7);
    add(c, 1);
    goto reduce_problem__reduce_b_loop__4;
reduce_problem__before_reduce_a_loop__4:
    solved_goto(c % 3, reduce_problem__move3__reduce_by_3__1__4);
    add(c, -1);
    add(b, 1);
    goto reduce_problem__before_reduce_a_loop__4;
reduce_problem__move3__reduce_by_3__1__4:
    solved_goto(c, reduce_problem__move3__after_move_loop__1__4);
    add(c, -3);
    add(b, 3);
    goto reduce_problem__move3__reduce_by_3__1__4;
reduce_problem__move3__after_move_loop__1__4:
    solved_goto(a, reduce_problem__after_reduce_a_loop__4);
    add(a, -1);
    add(c, 7);
    goto reduce_problem__move3__after_move_loop__1__4;
reduce_problem__after_reduce_a_loop__4:
    solved_goto(c % 10, reduce_problem__move10__reduce_by_10__1__4);
    add(c, -1);
    add(a, 1);
    goto reduce_problem__after_reduce_a_loop__4;
reduce_problem__move10__reduce_by_10__1__4:
    solved_goto(c, move3__after_move_loop__4);
    add(c, -10);
    add(a, 10);
    goto reduce_problem__move10__reduce_by_10__1__4;
reduce_generator_11:
    add(b, -1);
before_reduce_generator_11:
    solved_goto(b, move_const__after_move_const_loop__2);
    add(b, 1);
reduce_problem__reduce_b_loop__5:
    solved_goto(b, reduce_problem__before_reduce_a_loop__5);
    add(b, -11);
    add(c, 1);
    goto reduce_problem__reduce_b_loop__5;
reduce_problem__before_reduce_a_loop__5:
    solved_goto(c % 3, reduce_problem__move3__reduce_by_3__1__5);
    add(c, -1);
    add(b, 1);
    goto reduce_problem__before_reduce_a_loop__5;
reduce_problem__move3__reduce_by_3__1__5:
    solved_goto(c, reduce_problem__move3__after_move_loop__1__5);
    add(c, -3);
    add(b, 3);
    goto reduce_problem__move3__reduce_by_3__1__5;
reduce_problem__move3__after_move_loop__1__5:
    solved_goto(a, reduce_problem__after_reduce_a_loop__5);
    add(a, -1);
    add(c, 11);
    goto reduce_problem__move3__after_move_loop__1__5;
reduce_problem__after_reduce_a_loop__5:
    solved_goto(c % 10, reduce_problem__move10__reduce_by_10__1__5);
    add(c, -1);
    add(a, 1);
    goto reduce_problem__after_reduce_a_loop__5;
reduce_problem__move10__reduce_by_10__1__5:
    solved_goto(c, reduce_generator_11);
    add(c, -10);
    add(a, 10);
    goto reduce_problem__move10__reduce_by_10__1__5;
move_0_b:
    solved_goto(b, move_const__after_move_const_loop__2);
    add(b, -1);
    goto move_0_b;
move_0_a:
    solved_goto(a, move_const__after_move_const_loop__2);
    add(a, -1);
    goto move_0_a;
move_const__after_move_const_loop__2:
    return;
}

int main()
{
    char buf[100];
    for (int i = 0; i < 30; i++)
    {
        for (int j = 0; j < 30; j++)
        {
            int a = i;
            int b = j;
            int c = 0;
            multiply(&a, &b, &c);
            printf("%d * %d = %d\n", i, j, a);
            assert(((long long)i * (long long)j) % 30 == a);
            assert(b == 0);
            assert(c == 0);
        }
    }
}
