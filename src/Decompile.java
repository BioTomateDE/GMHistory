// Shitty script for invoking Ghidra's `analyzeHeadless` tool.
// Mostly slopcoded, sorry.

import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.scalar.Scalar;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.symbol.ReferenceIterator;
import java.io.*;
import java.nio.file.*;
import java.util.*;
import java.util.regex.Pattern;
import java.util.stream.Stream;

public class Decompile extends GhidraScript {
    private static Pattern BLOCK = Pattern.compile("(?m)/\\*+[ \\t]*(.*?\\S)[ \\t]*\\*+/[ \\t]*$");

    @Override
    protected void run() throws Exception {
        fixStrings();
        decompileAllAsC();
    }

    private Data findStringData(String value) throws Exception {
        for (Data d : currentProgram.getListing().getDefinedData(true)) {
            if (d.hasStringValue() && value.equals(d.getValue())) return d;
        }
        throw new Exception("Could not find string: " + value);
    }

    private Function findRegisterFunc() throws Exception {
        Data anchorData = findStringData("show_debug_message");
        List<Address> targets = new ArrayList<>();
        targets.add(anchorData.getAddress());

        // Pass 1: direct references. Pass 2: references to pointers that point at the string.
        for (int pass = 0; pass < 2; pass++) {
            List<Address> next = new ArrayList<>();
            for (Address target : targets) {
                for (Reference ref : getReferencesTo(target)) {
                    Address from = ref.getFromAddress();
                    Instruction ins = getInstructionContaining(from);
                    if (ins == null) {
                        // Data reference (pointer table entry): look for references to that pointer next
                        next.add(from);
                        continue;
                    }
                    Function f = callAfter(ins);
                    if (f != null) return f;
                }
            }
            targets = next;
        }

        // Fallback: scan for any pointer-sized value equal to the string address
        Memory m = currentProgram.getMemory();
        int psize = currentProgram.getDefaultPointerSize();
        byte[] pat = addrBytes(anchorData.getAddress(), psize);
        Address hit = m.findBytes(currentProgram.getMinAddress(), pat, null, true, monitor);
        while (hit != null) {
            println("raw pointer to string at " + hit);
            for (Reference ref : getReferencesTo(hit)) {
                Instruction ins = getInstructionContaining(ref.getFromAddress());
                if (ins == null) continue;
                Function f = callAfter(ins);
                if (f != null) return f;
            }
            Address nextStart = hit.add(1);
            hit = m.findBytes(nextStart, pat, null, true, monitor);
        }

        throw new Exception("Could not find GML function register function (string at "
                + anchorData.getAddress() + ", refs: "
                + getReferencesTo(anchorData.getAddress()).length + ")");
    }

    /** Walk forward within the same function until the first call with a resolvable target. */
    private Function callAfter(Instruction ins) {
        Function owner = getFunctionContaining(ins.getAddress());
        for (int i = 0; i < 24 && ins != null; i++) {
            if (owner != null && !owner.getBody().contains(ins.getAddress())) break;
            if (ins.getFlowType().isCall()) {
                for (Address flow : ins.getFlows()) {
                    Function f = getFunctionAt(flow);
                    if (f == null) f = createFunction(flow, null);  // target not analyzed as a function
                    if (f != null) return f;
                }
            }
            ins = ins.getNext();
        }
        return null;
    }

    private byte[] addrBytes(Address a, int size) {
        long v = a.getOffset();
        byte[] b = new byte[size];
        for (int i = 0; i < size; i++) b[i] = (byte) (v >> (8 * i));  // little-endian (x86)
        return b;
    }

    private void fixStrings() throws Exception {
        Function target = findRegisterFunc();
        println("Registration function: " + target.getName() + " @ " + target.getEntryPoint());
        int total = 0, fixed = 0;
        ReferenceIterator callRefs = currentProgram.getReferenceManager().getReferencesTo(target.getEntryPoint());
        for (Reference ref : callRefs) {
            Instruction callIns = getInstructionAt(ref.getFromAddress());
            if (callIns == null || !callIns.getFlowType().isCall()) continue;
            total++;

            Instruction pushIns = getInstructionBefore(callIns);
            if (pushIns == null || !pushIns.getMnemonicString().equalsIgnoreCase("PUSH")) continue;

            Address strAddr = pushIns.getAddress(0);
            if (strAddr == null) {
                Scalar sc = pushIns.getScalar(0);
                if (sc != null) {
                    try {
                        strAddr = toAddr(sc.getUnsignedValue());
                    } catch (Exception e) {
                        strAddr = null;
                    }
                }
            }
            if (strAddr == null) continue;

            Data existing = getDataAt(strAddr);
            if (existing != null && existing.hasStringValue()) continue; // already fine

            try {
                clearListing(strAddr, strAddr);
                createAsciiString(strAddr);
                fixed++;
            } catch (Exception e) {
                println("Skipped " + strAddr + ": " + e.getMessage());
            }
        }
        println("Calls found: " + total + ", strings fixed: " + fixed);
    }

    void decompileAsC(DecompInterface ifc, PrintWriter w, Function func) throws Exception {
        DecompileResults r = ifc.decompileFunction(func, 10, monitor);
        if (!r.decompileCompleted()) {
            String why = r.isTimedOut() ? "timed out" : String.valueOf(r.getErrorMessage());
            println("Failed to decompile " + func.getName() + ": " + why);
            w.println("// " + func.getName() + " @ " + func.getEntryPoint() + ": decompile failed (" + why + ")");
            return;
        }
        String code = r.getDecompiledFunction().getC();
        code = code.replace(")\n\n{", ") {");
        code = code.replace("\u0027\\0\u0027", "0");
        code = BLOCK.matcher(code).replaceAll("// $1");
        w.println(code);
    }

    void decompileAllAsC() throws Exception  {
        File out = new File(getScriptArgs()[0]);
        DecompInterface ifc = new DecompInterface();
        ifc.openProgram(currentProgram);

        try (PrintWriter w = new PrintWriter(out)) {
            for (Function f : currentProgram.getFunctionManager().getFunctions(true)) {
                if (f.isThunk() || f.isExternal()) continue;
                decompileAsC(ifc, w, f);
            }
        }
    }
}
