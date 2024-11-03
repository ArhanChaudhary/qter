U := ( 1, 3, 8, 6)( 2, 5, 7, 4)( 9,33,25,17)(10,34,26,18)(11,35,27,19);
L := ( 9,11,16,14)(10,13,15,12)( 1,17,41,40)( 4,20,44,37)( 6,22,46,35);
F := (17,19,24,22)(18,21,23,20)( 6,25,43,16)( 7,28,42,13)( 8,30,41,11);
R := (25,27,32,30)(26,29,31,28)( 3,38,43,19)( 5,36,45,21)( 8,33,48,24);
B := (33,35,40,38)(34,37,39,36)( 3, 9,46,32)( 2,12,47,29)( 1,14,48,27);
D := (41,43,48,46)(42,45,47,44)(14,22,30,38)(15,23,31,39)(16,24,32,40);
cube := Group(U, L, F, R, B, D);

moves := [U, L, F, R, B, D, U^-1, L^-1, F^-1, R^-1, B^-1, D^-1, U^2, L^2, F^2, R^2, B^2, D^2];;
for i in [1..10000] do
    s := [];
    for j in [1..15] do
        Add(s, RandomList([1..48]));
    od;
    s := Stabilizer(cube, s, OnTuples);
    if Size(s) > 10000000 then continue; fi;
    for j in Set(List(ConjugacyClasses(s), x -> Order(Representative(x)))) do
        a := ConjugacyClassesOfOrder(s, j);
        b := Filtered(ConjugacyClasses(s), x -> Order(Representative(x)) = j);
        if Length(a) <> Length(b) then
            Error("Test failed");
        fi;
        for j in a do
            if not j in b then
                Error("Test failed");
            fi;
        od;
        for j in b do
            if not j in a then
                Error("Test failed");
            fi;
        od;
    od;
    AppendTo("x.txt", "Test ", i, " passed\n");
od;
