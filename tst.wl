!@import w gamelib:wordgen;
!@import wl gamelib:wordlib;

!ucfirst = \std:str:cat[std:str:to_uppercase ~ _ 0 1, _ 1 -1];
!ucfirst_on_words = {
    std:re:map $q$(\S+)$ { ucfirst _.1 } _ | std:str:join " ";
};

!r = std:rand:split_mix64_new[];
!rand_gen = { std:rand:split_mix64_next_open01 r };

range 1 100 1 {||
    !name_pattern = w:gen "f s" wl:name_set rand_gen;
    w:gen name_pattern wl:particle_set rand_gen | ucfirst_on_words | std:displayln;
};

#!occ = ${};
#range 1 1000 1 {||
#    !res = w:gen "vvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvv" w:set1 rand_gen;
#    res { occ.(_) = occ.(_) + 1; }
#};
##0std:displayln occ;
#occ {
#    std:displayln @;
#};
