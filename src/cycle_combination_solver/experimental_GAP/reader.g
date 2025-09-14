# does not work; experimental

stream := InputOutputLocalProcess(DirectoryCurrent(), "../../vcube/vc-optimal", [ "--coord=404", "--format=speffz" ]);

U := ( 1, 3, 8, 6)( 2, 5, 7, 4)( 9,33,25,17)(10,34,26,18)(11,35,27,19);
L := ( 9,11,16,14)(10,13,15,12)( 1,17,41,40)( 4,20,44,37)( 6,22,46,35);
F := (17,19,24,22)(18,21,23,20)( 6,25,43,16)( 7,28,42,13)( 8,30,41,11);
R := (25,27,32,30)(26,29,31,28)( 3,38,43,19)( 5,36,45,21)( 8,33,48,24);
B := (33,35,40,38)(34,37,39,36)( 3, 9,46,32)( 2,12,47,29)( 1,14,48,27);
D := (41,43,48,46)(42,45,47,44)(14,22,30,38)(15,23,31,39)(16,24,32,40);
cube := Group(U, L, F, R, B, D);
edge_facelet_buf := 42;
corner_facelet_buf := 1;
edge_cubies := Blocks(cube, Orbit(cube, edge_facelet_buf));
corner_cubies := Blocks(cube, Orbit(cube, corner_facelet_buf));
edge_facelets := Immutable(Flat(edge_cubies));
corner_facelets := Immutable(Flat(corner_cubies));

Read("phase3/util.g");

outFile := OutputTextFile("out.txt", false);
for i in ConjugacyClass(cube, (1,3,6,9,33,17,35,27,11)(2,13,44,4,45,34,20,15,10,31)(7,18)(14,40,46)) do
    j := PermutationSpeffz(i);
    Add(j, '\n');
    WriteLine(stream, j);
    line := ReadAllLine(stream);
    if line <> fail then
        WriteAll(outFile, line);
    fi;
od;

# for i in c do
#     j := PermutationSpeffz(i);
#     AppendTo(outFile, '\n');
# od;

CloseStream(stream);
CloseStream(outFile);