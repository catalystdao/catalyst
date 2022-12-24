# This file contains the logic to compute the lookup tables used by the mathematical library

from math import exp
from typing import List

U256_MAX = 2**256-1



# Functions used to compute the lookup tables
# ! IMPORTANT ! THE FOLLOWING FUNCTIONS DO NOT YIELD ACCURATE RESULTS, UPDATE THEM WITH THE ORIGINAL FUNCTIONS USED TO DERIVE THE LOOKUP TABLES
# Alternative functions which do yield accurate tables (yet not the original functions) can be found in:
#   catalyst/solana/programs/TestProgramFixedPointMath/utils/lookupTablesGenerators.ts

# def compute_two_two_minus_i(term_count: int) -> List[int]:
#     return [ int(2**(2**(-(i+1)))*2**64) for i in range(term_count) ]

# def compute_two_minus_two_minus_i(term_count: int) -> List[int]:
#     return [ int(2**(-2**(-(i+1))+64)) for i in range(term_count) ]

# def compute_exp_pos(term_count: int) -> List[int]:
#     return [ int(exp(2**i)*2**64) for i in range(term_count) ]

# def compute_exp_neg(term_count: int) -> List[int]:
#     return [ int(exp(2**(-(i+1)))*2**64) for i in range(term_count) ]

# def compute_inv_exp_pos(term_count: int) -> List[int]:
#     return [ int(exp(-2**i)*2**64) for i in range(term_count) ]

# def compute_inv_exp_neg(term_count: int) -> List[int]:
#     return [ int(exp(-2**(-(i+1)))*2**64) for i in range(term_count) ]



# The lookup tables used

TWO_TWO_MINUS_I       : List[int] = [26087635650665564425, 21936999301089678047, 20116317054877281742, 19263451207323153962, 18850675170876015534, 18647615946650685159, 18546908069882975960, 18496758270674070881, 18471734244850835106, 18459234930309000272, 18452988445124272033, 18449865995240371898, 18448304968436414829, 18447524504564044946, 18447134285009651015, 18446939178327825412, 18446841625760745902, 18446792849670663277, 18446768461673986097, 18446756267687738522]
TWO_MINUS_TWO_MINUS_I : List[int] = [13043817825332782212, 15511800964685064948, 16915738899553466670, 17664662643191237676, 18051468387014017850, 18248035989933441396, 18347121020861646923, 18396865112328554661, 18421787711448657617, 18434261669329232139, 18440501815349552981, 18443622680442407997, 18445183311048607332, 18445963675871538003, 18446353870663572145, 18446548971154807802, 18446646522174239825, 18446695297877410579, 18446719685777359790, 18446731879739425374]
EXP_POS               : List[int] = [50143449209799256682, 136304026803256390412, 1007158100559408451354, 54988969081439155412736, 163919806582506698591828152, 1456609517792428406714055862390917, 115018199355157251870643531501709554553678249259, 717155619985916044695037432918736248907406552372215529479395529955709617329]
EXP_NEG               : List[int] = [30413539329486470295, 23686088245777032822, 20902899511243624348, 19636456851539679189, 19032306587872971150, 18737238023755501946, 18591423680622123788,  18518942588666869714, 18482808078018285842, 18464767271176066525, 18455753472345505796, 18451248223137477973, 18448996010967782912, 18447870007976831669, 18447307032252994902, 18447025550833756842, 18446884811734779221, 18446814442587947178, 18446779258115194901, 18446761665903984642]
INV_EXP_POS           : List[int] = [6786177901268885274, 2496495334008788799, 337863903126961437, 6188193243211692, 2075907567336, 233612, 0, 0]
INV_EXP_NEG           : List[int] = [11188515852577165299, 14366338729722795843, 16279194507819420732, 17329112349219823218, 17879197424118840458, 18160753814917686419, 18303190372430456779,  18374827034086858296, 18410750438167364677, 18428738468430479223, 18437739073120195921, 18442241023793258495, 18444492411329227605, 18445618208161748319, 18446181132345977515, 18446462600880313685, 18446603336758065834, 18446673705099591509, 18446708889371017194, 18446726481531895805]



# Utils

def get_rel_error(val: int , target: int) -> float:
    if val == 0 and target == 0:
        return 0

    return abs(2*(val - target)/(abs(val) + abs(target)))

def get_list_rel_error(list: List[int], target_list: List[int]) -> float:
    return sum([get_rel_error(el, target_el) for el, target_el in zip(list, target_list)])/len(list)


def int_to_u256_array_rep(value: int) -> List[int]:
    if value > U256_MAX:
        raise OverflowError("Unable to compute u256 representation for given value: overflow.")

    return [ (value >> 64*i) & 0xFFFFFFFFFFFFFFFF for i in range(4)]



if __name__ == '__main__':

    # Compare the hardcoded lookup tables with the generator functions

    # # print("\nHardcoded tables vs computed tables:\n")

    # calc_two_two_minus_i = compute_two_two_minus_i(20)
    # if calc_two_two_minus_i == TWO_TWO_MINUS_I:
    #     print("TWO_TWO_MINUS_I: match ✅")
    # else:
    #     print(f"TWO_TWO_MINUS_I: mismatch ❌, avg. error: {[get_list_rel_error(calc_two_two_minus_i, TWO_TWO_MINUS_I)]}")

    # calc_two_minus_two_minus_i = compute_two_minus_two_minus_i(20)
    # if calc_two_minus_two_minus_i == TWO_MINUS_TWO_MINUS_I:
    #     print("TWO_MINUS_TWO_MINUS_I: match ✅")
    # else:
    #     print(f"TWO_MINUS_TWO_MINUS_I: mismatch ❌, avg. error: {[get_list_rel_error(calc_two_minus_two_minus_i, TWO_MINUS_TWO_MINUS_I)]}")

    # calc_exp_pos = compute_exp_pos(8)
    # if calc_exp_pos == EXP_POS:
    #     print("EXP_POS: match ✅")
    # else:
    #     print(f"EXP_POS: mismatch ❌, avg. error: {[get_list_rel_error(calc_exp_pos, EXP_POS)]}")

    # calc_exp_neg = compute_exp_neg(20)
    # if calc_exp_neg == EXP_NEG:
    #     print("EXP_NEG: match ✅")
    # else:
    #     print(f"EXP_NEG: mismatch ❌, avg. error: {[get_list_rel_error(calc_exp_neg, EXP_NEG)]}")

    # calc_inv_exp_pos = compute_inv_exp_pos(8)
    # if calc_inv_exp_pos == INV_EXP_POS:
    #     print("INV_EXP_POS: match ✅")
    # else:
    #     print(f"INV_EXP_POS: mismatch ❌, avg. error: {[get_list_rel_error(calc_inv_exp_pos, INV_EXP_POS)]}")

    # calc_inv_exp_neg = compute_inv_exp_neg(20)
    # if calc_inv_exp_neg == INV_EXP_NEG:
    #     print("INV_EXP_NEG: match ✅")
    # else:
    #     print(f"INV_EXP_NEG: mismatch ❌, avg. error: {[get_list_rel_error(calc_inv_exp_neg, INV_EXP_NEG)]}")
    
    # print("")

    # # print(compute_two_two_minus_i(20))


    # # Formatted tables for Solana (u256 as u64 array)
    print(f"TWO_TWO_MINUS_I: [{', '.join([f'U256({int_to_u256_array_rep(el)})' for el in TWO_TWO_MINUS_I])}]")
    print(f"TWO_MINUS_TWO_MINUS_I: [{', '.join([f'U256({int_to_u256_array_rep(el)})' for el in TWO_MINUS_TWO_MINUS_I])}]")
    print(f"EXP_POS: [{', '.join([f'U256({int_to_u256_array_rep(el)})' for el in EXP_POS])}]")
    print(f"EXP_NEG: [{', '.join([f'U256({int_to_u256_array_rep(el)})' for el in EXP_NEG])}]")
    print(f"INV_EXP_POS: [{', '.join([f'U256({int_to_u256_array_rep(el)})' for el in INV_EXP_POS])}]")
    print(f"INV_EXP_NEG: [{', '.join([f'U256({int_to_u256_array_rep(el)})' for el in INV_EXP_NEG])}]")



