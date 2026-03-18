(function() {
    const PREFIX = '__domscan';
    const CANARY = window.__CANARY;

    function logEvent(
        detection_type,
        name,
        value,
        result,
    ) {
        window.__inspector_callback({
            detection_type: detection_type,
            stack: new Error().stack.split('\n').slice(2).join('\n').trim(),
            name: name,
            value: value,
            result: result
        });
    }
    
    // ---- Sources hooking
    // location.* & document.URL -> property not configurable)
    const sources = [
        //{ name: 'Storage.getItem', obj: Storage.prototype, prop: 'getItem' },
        //{ name: 'Storage.setItem', obj: Storage.prototype, prop: 'setItem' },
        //{ name: 'Storage.removeItem', obj: Storage.prototype, prop: 'removeItem' },

        { name: 'URLSearchParams.get',    obj: URLSearchParams.prototype, prop: 'get'    },
        { name: 'URLSearchParams.getAll', obj: URLSearchParams.prototype, prop: 'getAll' },
        { name: 'URLSearchParams.has',    obj: URLSearchParams.prototype, prop: 'has'    },
        { name: 'URLSearchParams.forEach',obj: URLSearchParams.prototype, prop: 'forEach'},
    ];

    function hookSource({ obj, prop, name }) {
        const descriptor = Object.getOwnPropertyDescriptor(obj, prop);
        if (!descriptor) {
            console.warn(`${PREFIX} → ${name} : descriptor unavailable`);
            return;
        }

        if (descriptor.configurable === false) {
            console.warn(`${PREFIX} → skip ${name} (not configurable)`);
            return;
        }

        try {
            if (descriptor.get) {
                Object.defineProperty(obj, prop, {
                    ...descriptor,
                    get: function () {
                        const value = descriptor.get.call(this);
                        logEvent("source.get", `${obj}`, prop.toString(), value);
                        return value;
                    }
                });
            } else if (typeof descriptor.value === 'function') {
                const original = descriptor.value;
                Object.defineProperty(obj, prop, {
                    ...descriptor,
                    value: function (...args) {
                        const result = original.apply(this, args);
                        const argStr = args.map(a => JSON.stringify(a)).join(', ');
                        logEvent("source.call", `${name}`, `${argStr}`, result);
                        return result;
                    }
                });
            }
        } catch (e) {
            console.error(`error: ${name} : ${e.message}`);
        }
    }

    sources.forEach(hookSource);
    
    const originalSplit = String.prototype.split;
    String.prototype.split = function(separator, limit) {
        const result = originalSplit.apply(this, arguments);
        if (separator === "=" && result.length === 2) {
            const thisStr = String(this);
            logEvent('source.manual-url-parse', `String.split('${separator}')`, thisStr, result[1]);
        }
        return result;
    };

    // ---- sink hooking
    const sinks = [
        //  Vanilla JS
        { name: 'innerHTML',          target: Element.prototype,   prop: 'innerHTML'          },
        { name: 'outerHTML',          target: Element.prototype,   prop: 'outerHTML'          },
        { name: 'insertAdjacentHTML', target: Element.prototype,   prop: 'insertAdjacentHTML' },
        { name: 'document.write',     target: Document.prototype,  prop: 'write'              },
        // eval runs "forever" for some reason...
        //{ name: 'eval',               target: window,              prop: 'eval'               },
        { name: 'setTimeout',         target: window,              prop: 'setTimeout'         },
        { name: 'setInterval',        target: window,              prop: 'setInterval'        },
        { name: 'Function',           target: window,              prop: 'Function'           },
        //  React
        { name: 'dangerouslySetInnerHTML', special: true }
    ];
    
    sinks.forEach(sink => {
        if (sink.special) return; // handled separately

        const descriptor = Object.getOwnPropertyDescriptor(sink.target, sink.prop);
        if (!descriptor || descriptor.configurable === false) return;

        try {
            if (descriptor.set) {
                Object.defineProperty(sink.target, sink.prop, {
                    ...descriptor,
                    set(value) {
                        logEvent('sink.set', sink.name, value, "");
                        return descriptor.set.call(this, value);
                    }
                });
            } else if (typeof descriptor.value === 'function') {
                const original = descriptor.value;
                Object.defineProperty(sink.target, sink.prop, {
                    ...descriptor,
                    value(...args) {
                        if (typeof args[0] === 'string') {
                            logEvent('sink.call', sink.name, args[0], "");
                        }
                        return original.apply(this, args);
                    }
                });
            }
        } catch (e) {
            console.warn(`${PREFIX} → skip sink ${sink.name}: ${e.message}`);
        }
    });
})();