OPERATOR_IMG = 'probelet/operator'
local_resource('compile-operator', 'just compile operator')
docker_build(OPERATOR_IMG, 'crates/operator')
k8s_yaml(helm('./charts/operator', set=['image.repository=' + OPERATOR_IMG]))
k8s_resource('chart-operator', port_forwards=8080)
