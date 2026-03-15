(function() {
    const PREFIX = 'soDOMx-src';

    // location.* & document.URL -> property not configurable)
    const sources = [
        { name: 'Storage.getItem', obj: Storage.prototype, prop: 'getItem' },
        { name: 'Storage.setItem', obj: Storage.prototype, prop: 'setItem' },
        { name: 'Storage.removeItem', obj: Storage.prototype, prop: 'removeItem' },

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
                        console.log(`${PREFIX} → ${name} = ${JSON.stringify(value)}`);
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
                        console.log(`${PREFIX} → ${name}(${argStr}) = ${JSON.stringify(result)}`);
                        return result;
                    }
                });
            }
        } catch (e) {
            console.error(`error: ${name} : ${e.message}`);
        }
    }

    sources.forEach(hookSource);
})();