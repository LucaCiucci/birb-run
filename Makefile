

car.yaml: engine.yaml plan.txt
	echo "This is a car recipe"
	echo "It generates a car.yaml file"
	echo car.yaml > car.yaml

engine.yaml: workbench.yaml engine_plan.txt
	echo "Building engine with configuration {{ configuration }}"
	echo "cfg: {{ configuration }}" > engine.yaml

workbench.yaml:
	echo "Workbench prepared at " $(date) > workbench.yaml

#.PHONY: workbench.yaml
