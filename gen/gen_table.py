import requests
import sys


def main():
    # File having contents from: https://gbdev.io/gb-opcodes/Opcodes.json
    try:
        data = requests.get("https://gbdev.io/gb-opcodes/Opcodes.json")
    except:
        print("Error downloading instructions info file.")
        sys.exit(1)

    j = data.json()

    # Just copy paste these...
    print_syntax(j["unprefixed"])
    print("--------------------------------------------------------------")
    print_syntax(j["cbprefixed"])


def operand_to_syntax(name: str, insn: str, is_addr):
    r = "Op::"

    # And address type is just U16, it is actually an
    # address only if is_addr is True
    name = name.replace("a", "n")

    # Along with matching also verify that the register names being used
    # are available for the addressing mode being used.
    match name:
        case 'n8':
            r += f"A8(0)" if is_addr else f"U8(0)"
        case 'e8':
            r += f"I8(0)"

        case 'n16':
            r += f"A16(0)" if is_addr else f"U16(0)"

       # Conditions can only appear in these,
       # needed for 'C' as it is both: condition and R8.
        case 'NZ' | 'Z' | 'NC' | 'C' if insn in ["JR", "JP", "RET", "CALL"]:
            r += f"Cond(Cond::{name})"

        # These names are available for direct addressing.
        case 'A' | 'F' | 'B' | 'C' | 'D' | 'E' | 'H' | 'L'\
                | 'AF' | 'BC' | 'DE' | 'HL' | 'SP' if not is_addr:
            r += f"Reg(Reg::{name})"

        # These names are available for indirect addressing.
        case 'A' | 'C' | 'BC' | 'DE' | 'HL' | 'HLinc' | 'HLdec' if is_addr:
            r += f"RegMem(Reg::{name})"

        # For: LD HL, SP, e8
        case 'SPinc':
            r += "SPplusI8(0)"

        # For: TGT $xx
        case v if v[0] == '$':
            r += f"Tgt(0x{v[1:]})"

        # For CB prefixed bit index instructions
        case v if v.isdigit():
            r += f"B3({v})"

        case _:
            raise ValueError(f"Unknown operand name: '{name}'")

    return r


def print_syntax(instructions):
    for op, info in instructions.items():
        name = info["mnemonic"]
        cycles = info["cycles"]
        operands = info["operands"]
        ops = []

        for opd in operands:
            n = opd["name"]
            if opd.get("increment"):
                n += "inc"
            elif opd.get("decrement"):
                n += "dec"

            ops.append(operand_to_syntax(n, name, not opd["immediate"]))

        if name.startswith("ILLEGAL"):
            name = "ILLEGAL"

        # Change LD to LDH when '[C]' or '[A8]' is its operand.
        # It will work even if we used LD but since address is obtained
        # via 0xFF00 + C|A8, it makes more sense to use LDH.
        # The same is given in several sources and feels uniform.
        if name == "LD" and ("Op::Cmem" in ops or "Op::A8(0)" in ops):
            name = "LDH"

        # Discard extra generated arg(Op::I8) for: LD HL, SP, e8.
        if name == "LD" and len(ops) == 3:
            ops.pop()

        # Stringify cycles and operand syntax fragments
        rest = ", ".join(ops)
        if rest != "":
            rest = ", " + rest

        # For Rust syntax, add number of cycles taken as comment.
        print(f"a[{op}] = ins!({name.title()}{rest}); // #{cycles}")


if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        print("Exception:", e)
        sys.exit(1)
