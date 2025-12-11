
import { buildSubnetPlanFromSeedIp } from '../web/src/devices/scan-subnet.ts'; // We might need to handle TS import via some loader or just copy-paste for a quick script if environment is plain node. 
// Wait, "web/src/devices/scan-subnet.ts" is TypeScript. Node won't run it directly without loader.
// The user suggestion was "scripts/test-scan-helpers.mjs" and "Use Node to call compiled JS" OR "Playwright".
// Since we are in a vite project, maybe `bun` or `ts-node` is available? 
// User mentioned "bun run dev". So `bun` is likely available! Bun runs TS natively.
// Let's try to assume `bun` is available.

import assert from 'node:assert';
import { test } from 'node:test'; // Bun supports CJS/ESM mixed, but node:test might be specific.
// Let's just write a simple main function if node:test is not available, 
// but Bun usually runs files directly. 

console.log("Running subnet logic tests...");

function runTests() {
    // 1. Valid private IP
    {
        const seed = "192.168.1.42";
        console.log(`Testing valid seed: ${seed}`);
        const plan = buildSubnetPlanFromSeedIp(seed);
        assert.strictEqual(plan.cidr, "192.168.1.0/24");
        assert.strictEqual(plan.hosts.length, 254);
        assert.strictEqual(plan.hosts[0], "192.168.1.1");
        assert.strictEqual(plan.hosts[253], "192.168.1.254");
    }

    // 2. Valid 10.x.x.x
    {
        const seed = "10.0.0.5";
        console.log(`Testing valid seed: ${seed}`);
        const plan = buildSubnetPlanFromSeedIp(seed);
        assert.strictEqual(plan.cidr, "10.0.0.0/24");
        assert.strictEqual(plan.hosts.length, 254);
    }

    // 3. Invalid format
    const invalidInputs = ["", "abc", "192.168.1", "192.168.1.1.1", "256.0.0.1", "-1.0.0.0"];
    for (const input of invalidInputs) {
        try {
            buildSubnetPlanFromSeedIp(input);
            console.error(`FAILED: Should have thrown for ${input}`);
            process.exit(1);
        } catch (e) {
            console.log(`Correctly threw error for ${input}: ${e.message}`);
        }
    }

    console.log("ALL TESTS PASSED");
}

runTests();
