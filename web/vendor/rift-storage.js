// A tiny, self-contained macroquad plugin: it gives the WASM two functions to
// read and write a single number (RIFT's best depth reached) in the browser's
// localStorage. It passes plain numbers, so it needs no sapp_jsutils and can't
// fall out of step with the macroquad bundle. Native builds persist the same
// value to a file instead; see the breakout-storage crate.
(function () {
    var KEY = "rift_best_depth";
    function register(importObject) {
        importObject.env.rift_storage_get = function () {
            try {
                var value = localStorage.getItem(KEY);
                return value === null ? 0 : parseFloat(value) || 0;
            } catch (e) {
                return 0;
            }
        };
        importObject.env.rift_storage_set = function (value) {
            try {
                localStorage.setItem(KEY, value);
            } catch (e) {
                /* private-mode or disabled storage: silently skip */
            }
        };
    }
    miniquad_add_plugin({
        register_plugin: register,
        on_init: function () {},
        name: "rift_storage",
        version: "1",
    });
})();
