LoadPackage("datastructures");

U := ( 1, 3, 8, 6)( 2, 5, 7, 4)( 9,33,25,17)(10,34,26,18)(11,35,27,19);;
L := ( 9,11,16,14)(10,13,15,12)( 1,17,41,40)( 4,20,44,37)( 6,22,46,35);;
F := (17,19,24,22)(18,21,23,20)( 6,25,43,16)( 7,28,42,13)( 8,30,41,11);;
R := (25,27,32,30)(26,29,31,28)( 3,38,43,19)( 5,36,45,21)( 8,33,48,24);;
B := (33,35,40,38)(34,37,39,36)( 3, 9,46,32)( 2,12,47,29)( 1,14,48,27);;
D := (41,43,48,46)(42,45,47,44)(14,22,30,38)(15,23,31,39)(16,24,32,40);;
cube := Group(U, L, F, R, B, D);;
moves := [U, L, F, R, B, D, U^-1, L^-1, F^-1, R^-1, B^-1, D^-1, U^2, L^2, F^2, R^2, B^2, D^2];;
len_moves := Length(moves);
moveStrings := ["U", "L", "F", "R", "B", "D", "U'", "L'", "F'", "R'", "B'", "D'", "U2", "L2", "F2", "R2", "B2", "D2"];
FindSequenceWithCycleStructure := function(moves, target_cycle_structure)
    local queue, current_seq, perm, cycle_structure, prev_depth, move,
          current_depth, seen_perms, found_at_current_depth, last_move_index_mod, i;
    queue := PlistDeque();
    seen_perms := HashSet();
    PushBack(queue, [ moves[1] ]);
    prev_depth := 0;
    found_at_current_depth := false;
    while not IsEmpty(queue) do
        current_seq := PlistDequePopFront(queue);
        current_depth := Length(current_seq);
        perm := Product(current_seq);

        if perm in seen_perms then
            continue;
        fi;
        AddSet(seen_perms, perm);

        cycle_structure := CycleStructurePerm(perm);
        if current_depth > prev_depth then
            if found_at_current_depth then
                break;
            fi;
            Print("Searching depth: ", current_depth, "\n");
            prev_depth := current_depth;
        fi;
        if cycle_structure = target_cycle_structure then
            Print("Found sequence: ");
            for move in current_seq do
                Print(moveStrings[Position(moves, move)], " ");
            od;
            Print(" at depth ", current_depth, "\n");
            found_at_current_depth := true;
        fi;
        if not found_at_current_depth then
            # for move in moves do
            #     if current_seq[current_depth] in [move, move^-1, move^2] then
            #         continue;
            #     fi;
            #     PlistDequePushBack(queue, Concatenation(current_seq, [move]));
            # od;
            last_move_index_mod := Position(moves, current_seq[current_depth]) mod 6;
            for i in [1..len_moves] do
                if (last_move_index_mod = 2 and i mod 6 = 4) or
                    (last_move_index_mod = 1 and i mod 6 = 0) or
                    (last_move_index_mod = 3 and i mod 6 = 5) or
                    (last_move_index_mod = i mod 6)
                then
                    continue;
                fi;
                PlistDequePushBack(queue, Concatenation(current_seq, [moves[i]]));
            od;
        fi;
    od;
end;

FindSequenceWithCycleStructure(moves, [1, 1,,, 1,, 1]);

# ( 6,43,46, 8,38,17,24,40,25,48,11,30,14,19,32)( 7,18)(12,21,23,39,20,45,44,37,28,42,47,13,31,15)
# for repr in conj_classes do
#     for i in [1..depth] do
#         for move in moves do
#             conjugate := repr ^ move;
#             cycle_structure := CycleStructurePerm(conjugate);
#             if cycle_structure = target_cycle_structure then
#                 AppendTo("~/Desktop/temp.txt", "Found matching conjugate ",  repr, " with cycle structure: ", repr, "^", move, "\n");
#             fi;
#         od;
#     od;
# od;

# LoadPackage("datastructures");

# # Define cube moves (as permutations of cubies)
# U := ( 1, 3, 8, 6)( 2, 5, 7, 4)( 9,33,25,17)(10,34,26,18)(11,35,27,19);;
# L := ( 9,11,16,14)(10,13,15,12)( 1,17,41,40)( 4,20,44,37)( 6,22,46,35);;
# F := (17,19,24,22)(18,21,23,20)( 6,25,43,16)( 7,28,42,13)( 8,30,41,11);;
# R := (25,27,32,30)(26,29,31,28)( 3,38,43,19)( 5,36,45,21)( 8,33,48,24);;
# B := (33,35,40,38)(34,37,39,36)( 3, 9,46,32)( 2,12,47,29)( 1,14,48,27);;
# D := (41,43,48,46)(42,45,47,44)(14,22,30,38)(15,23,31,39)(16,24,32,40);;
# cube := Group(U, L, F, R, B, D);;
# # moves := [U, L, F, R, B, D, U^-1, L^-1, F^-1, R^-1, B^-1, D^-1, U^2, L^2, F^2, R^2, B^2, D^2];;

# symmetries := [U^2*L^2*F^2*D^2*U^2*F^2*R^2*U^2,
# B*F*L*R*B^-1*F^-1*D^-1*U^-1*L*R*D*U,
# U*L*D*U*L^-1*D^-1*U^-1*R*B^2*U^2*B^2*L^-1*R^-1*U^-1,
# U*L^-1*R^-1*B^2*U^-1*R^2*B*L^2*D^-1*F^2*L^-1*R^-1*U^-1,
# D*B*D*U^2*B^2*F^2*L^2*R^2*U^-1*F*U,
# B^-1*D^-1*U*L^-1*R*B^-1*F*U,
# L^-1*R*U^2*R^2*D^2*F^2*L*R*D^2,
# U^2*D^2,
# U*D,
# D^2,
# U*D^-1,
# D,
# U*R^2*L^2*U^2*R^2*L^2*D,
# U*F^2*B^2*D^2*F^2*B^2*U,
# U*R*L*F^2*B^2*R^-1*L^-1*U,
# U*R^2*L^2*D^2*F^2*B^2*U,
# B^2*D^2*U^2*F^2,
# U*F^2*U^2*D^2*F^2*D,
# R^2*L^2*F*B,
# U*R^2*L^2*U^2*F^2*B^2*U^-1,
# R^2*L^2*U^2,
# B^2*R^2*B^2*R^2*B^2*R^2,
# U^-1*D*F^2*B^2,
# U*R^2*U*D*R^2*D,
# L*R*U^2,
# U*R^2*D^-1*U^-1*R^2*U^-1,
# F^2*R^2,
# U*B^2*U*D*B^2*D^-1,
# U*D^-1*R*L^-1,
# U*R];

# # Define a function to apply all 48 symmetries and return the canonical form of a permutation
# CanonicalForm := function(perm)
#     local i, sym_perm, canonical;
#     canonical := perm;
#     for i in [1..30] do
#         sym_perm := symmetries[i]^-1 * perm * symmetries[i];
#         if sym_perm < canonical then
#             canonical := sym_perm;
#         fi;
#     od;
#     return canonical;
# end;

# FindSequenceWithCycleStructure := function(moves, target_cycle_structure)
#     local queue, current_seq, perm, cycle_structure, next_seq, move, prev_depth, current_depth, seen_cubes;
#     queue := PlistDeque();
#     seen_cubes := HashSet();
#     PushBack(queue, [R]);
#     prev_depth := 0;
#     while not IsEmpty(queue) do
#         current_seq := PlistDequePopFront(queue);
#         current_depth := Length(current_seq);
#         perm := Product(current_seq);
#         if current_depth <> 0 then
#             cycle_structure := CycleStructurePerm(perm);
#             if current_depth > prev_depth then
#                 Print("Searching depth: ", current_depth, "\n");
#                 prev_depth := current_depth;
#             fi;
#             if cycle_structure = target_cycle_structure then
#                 Print("Found sequence: ", current_seq, " at depth ", current_depth, "\n");
#                 AppendTo("~/Desktop/temp.txt", "Found sequence: ", current_seq, " at depth ", current_depth, "\n");
#             fi;
#         fi;
#         perm := CanonicalForm(perm);
#         if not perm in seen_cubes then
#             AddSet(seen_cubes, perm);

#             for move in moves do
#                 if current_seq[current_depth] = move or current_seq[current_depth] = move^-1 or current_seq[current_depth] = move^2 then
#                     continue;
#                 fi;
#                 next_seq := Concatenation(current_seq, [move]);
#                 PlistDequePushBack(queue, next_seq);
#             od;
#         fi;
#     od;
# end;

# # Run the search with symmetry reduction
# FindSequenceWithCycleStructure(moves, CycleStructurePerm(U^-1*F^-1*R^2*F^-1*R*U*F^-1*L*F*L^-1*U^-1*R*F));
