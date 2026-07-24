// A tiny, self-contained macroquad plugin: it gives the WASM two functions to
// read and write numbered slots (HAILFALL's saved numbers — the best Onslaught
// score, and the Daily day and best) in the browser's localStorage. It passes
// plain numbers, so it needs no sapp_jsutils and can't fall out of step with the
// macroquad bundle. Native builds persist the same slots to a file instead; see
// the stepfall-storage crate. The Faithful never calls it; only HAILFALL does.
(function () {
    function key(slot) {
        return "stepfall_slot_" + slot;
    }
    function register(importObject) {
        importObject.env.stepfall_storage_get = function (slot) {
            try {
                var value = localStorage.getItem(key(slot));
                return value === null ? 0 : parseFloat(value) || 0;
            } catch (e) {
                return 0;
            }
        };
        importObject.env.stepfall_storage_set = function (slot, value) {
            try {
                localStorage.setItem(key(slot), value);
            } catch (e) {
                /* private-mode or disabled storage: silently skip */
            }
        };
    }
    miniquad_add_plugin({
        register_plugin: register,
        on_init: function () {},
        name: "stepfall_storage",
        version: "1",
    });
})();
