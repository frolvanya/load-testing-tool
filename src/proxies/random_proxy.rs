use rand::Rng;

pub fn take_random_proxy(proxies: Vec<String>) -> String {
    let mut rng = rand::thread_rng();

    let rand_index = rng.gen_range(0..proxies.len());
    proxies[rand_index].clone()
}
