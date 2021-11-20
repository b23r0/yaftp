fn check_support_methods(methods : &[u8]) -> bool {
	
	let mut i = 0 ;
	while i < methods.len() {
		if methods[i] > 0x0a {
			return false;
		}
		i += 1;
	}
	
	true
}