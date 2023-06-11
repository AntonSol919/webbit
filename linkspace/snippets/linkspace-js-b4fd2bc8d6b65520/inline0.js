
export function identity(a) { return a }
export function set_iter(obj) {
     obj[Symbol.iterator] = function () { return this };
};
export function pkt_obj(pkt){
     return {
group:pkt.group,
domain:pkt.domain,
path: pkt.path,
links:pkt.links,
create:pkt.create
};
};
