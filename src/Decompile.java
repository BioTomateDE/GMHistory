// Shitty script for invoking Ghidra's `analyzeHeadless` tool.
// Mostly slopcoded.

import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.scalar.Scalar;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;
import java.io.*;
import java.nio.file.*;
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
        ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(anchorData.getAddress());
        for (Reference ref : refs) {
            Instruction ins = getInstructionAt(ref.getFromAddress());
            for (int i = 0; i < 6 && ins != null; i++) {
                if (ins.getFlowType().isCall() && ins.getFlows().length > 0) {
                    return getFunctionAt(ins.getFlows()[0]);
                }
                ins = getInstructionAfter(ins);
            }
        }
        throw new Exception("Could not find GML function register function");
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
