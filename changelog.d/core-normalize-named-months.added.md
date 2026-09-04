Dates printed with a month name are now normalised. Travel documents print dates as `14 OCT /OUT
2000` rather than `2000-10-14`, often bilingually, and every such date previously passed through
unrecognised. Month names are matched whole-token in the languages the corpus contains, and a
string naming two *different* months is left alone rather than guessed at — French `AOUT`
(August) ends with Portuguese `OUT` (October), and a substring match would turn every French
August into a well-formed wrong date.
