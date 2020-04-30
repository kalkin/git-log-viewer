''' Icon defintion consisting of regex + icon '''

ASCII = [
    (r'^Revert:?\s*', 'R '),
    (r'^fixup!\s+', 'f '),
    (r'^(hot|bug)?fix(ing|ed)?(\(.+\))?[\/:\s]+', 'B '),  # B for bug
    (r'^add(ed)?(:|\s)', '+ '),
    (r'^feat:?\s*', '+ '),
    (r'^build:?\s*', 'b '),
    (r'^doc(s|umentation)?:?\s*', 'D '),
    (r'^style:?\s*', 's '),
    (r'^test(\(.+\))?:?\s*', 'T '),
]
