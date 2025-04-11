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
 * - Argument 2: 0
 * - Argument 3: 0
 *
 * Caveats: The program is faster when the first argument is larger than the
 * second.
 */

#include <stdio.h>
#include <assert.h>

#define solved_goto(a, label) \
    if (*a == 0)              \
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
    solved_goto(a, l1);
    solved_goto(b, l2);
l3:
    solved_goto(b % 10, l4);
    add(b, -1);
    add(c, 1);
    solved_goto(b % 10, l5);
    add(b, -1);
    add(c, 1);
    goto l3;
l4:
    solved_goto(c % 3, l6);
    add(c, -1);
    add(b, 1);
    goto l4;
l6:
    solved_goto(c, l7);
    add(c, -3);
    add(b, 3);
    goto l6;
l7:
    solved_goto(b, l8);
    add(b, -2);
    add(c, 1);
    goto l7;
l8:
    solved_goto(c % 3, l9);
    add(c, -1);
    add(b, 1);
    goto l8;
l9:
    solved_goto(c, l10);
    add(c, -3);
    add(b, 3);
    goto l9;
l10:
    solved_goto(a, l11);
    add(a, -1);
    add(c, 2);
    goto l10;
l11:
    solved_goto(c % 10, l12);
    add(c, -1);
    add(a, 1);
    goto l11;
l12:
    solved_goto(c, l3);
    add(c, -10);
    add(a, 10);
    goto l12;
l13:
    solved_goto(c, l14);
    add(b, 1);
    add(c, -1);
    goto l13;
l14:
    solved_goto(b, l15);
    add(b, -3);
    add(c, 1);
    goto l14;
l15:
    solved_goto(c % 3, l16);
    add(c, -1);
    add(b, 1);
    goto l15;
l16:
    solved_goto(c, l17);
    add(c, -3);
    add(b, 3);
    goto l16;
l17:
    solved_goto(a, l18);
    add(a, -1);
    add(c, 3);
    goto l17;
l18:
    solved_goto(c % 10, l19);
    add(c, -1);
    add(a, 1);
    goto l18;
l19:
    solved_goto(c, l20);
    add(c, -10);
    add(a, 10);
    goto l19;
l5:
    solved_goto(c % 3, l21);
    add(c, -1);
    add(b, 1);
    goto l5;
l21:
    solved_goto(c, l20);
    add(c, -3);
    add(b, 3);
    goto l21;
l20:
    solved_goto(b % 3, l13);
    add(b, -1);
    solved_goto(b, l42);
    goto l49;
l22:
    solved_goto(b % 10, l23);
    add(b, -1);
l49:
    add(c, 1);
    solved_goto(b % 10, l24);
    add(b, -1);
    add(c, 1);
    solved_goto(b % 10, l24);
    add(b, -1);
    add(c, 1);
    solved_goto(b % 10, l24);
    add(b, -1);
    add(c, 1);
    solved_goto(b % 10, l24);
    add(b, -1);
    add(c, 1);
    goto l22;
l23:
    solved_goto(c % 3, l25);
    add(c, -1);
    add(b, 1);
    goto l23;
l25:
    solved_goto(c, l26);
    add(c, -3);
    add(b, 3);
    goto l25;
l26:
    solved_goto(b, l27);
    add(b, -5);
    add(c, 1);
    goto l26;
l27:
    solved_goto(c % 3, l28);
    add(c, -1);
    add(b, 1);
    goto l27;
l28:
    solved_goto(c, l29);
    add(c, -3);
    add(b, 3);
    goto l28;
l29:
    solved_goto(a, l30);
    add(a, -1);
    add(c, 5);
    goto l29;
l30:
    solved_goto(c % 10, l31);
    add(c, -1);
    add(a, 1);
    goto l30;
l31:
    solved_goto(c, l22);
    add(c, -10);
    add(a, 10);
    goto l31;
l24:
    solved_goto(c % 3, l32);
    add(c, -1);
    add(b, 1);
    goto l24;
l32:
    solved_goto(c, l33);
    add(c, -3);
    add(b, 3);
    goto l32;
l33:
    add(b, -1);
    solved_goto(b % 10, l34);
    add(b, 1);
l35:
    solved_goto(b, l36);
    add(b, -7);
    add(c, 1);
    goto l35;
l36:
    solved_goto(c % 3, l37);
    add(c, -1);
    add(b, 1);
    goto l36;
l37:
    solved_goto(c, l38);
    add(c, -3);
    add(b, 3);
    goto l37;
l38:
    solved_goto(a, l39);
    add(a, -1);
    add(c, 7);
    goto l38;
l39:
    solved_goto(c % 10, l40);
    add(c, -1);
    add(a, 1);
    goto l39;
l40:
    solved_goto(c, l33);
    add(c, -10);
    add(a, 10);
    goto l40;
l41:
    add(b, -1);
l34:
    solved_goto(b, l42);
    add(b, 1);
l43:
    solved_goto(b, l44);
    add(b, -11);
    add(c, 1);
    goto l43;
l44:
    solved_goto(c % 3, l45);
    add(c, -1);
    add(b, 1);
    goto l44;
l45:
    solved_goto(c, l46);
    add(c, -3);
    add(b, 3);
    goto l45;
l46:
    solved_goto(a, l47);
    add(a, -1);
    add(c, 11);
    goto l46;
l47:
    solved_goto(c % 10, l48);
    add(c, -1);
    add(a, 1);
    goto l47;
l48:
    solved_goto(c, l41);
    add(c, -10);
    add(a, 10);
    goto l48;
l1:
    solved_goto(b, l42);
    add(b, -1);
    goto l1;
l2:
    solved_goto(a, l42);
    add(a, -1);
    goto l2;
l42:
    return;
}

int main()
{
    for (int i = 0; i < 30; i++)
    {
        for (int j = 0; j < 30; j++)
        {
            int a = i;
            int b = j;
            int c = 0;
            multiply(&a, &b, &c);
            printf("%d * %d = %d\n", i, j, a);
            assert((i * j) % 30 == a);
            assert(b == 0);
            assert(c == 0);
        }
    }
}
