import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/auth/providers.dart';
import 'setup_steps.dart';

class SetupPage extends ConsumerStatefulWidget {
  const SetupPage({super.key, this.initialStep = 0});

  final int initialStep;

  @override
  ConsumerState<SetupPage> createState() => _SetupPageState();
}

class _SetupPageState extends ConsumerState<SetupPage> {
  late int _step;

  @override
  void initState() {
    super.initState();
    _step = widget.initialStep.clamp(0, 1);
  }

  @override
  Widget build(BuildContext context) {
    final hasServerUrl = ref.watch(authConfigProvider).value?.hasServerUrl ?? false;
    final step = hasServerUrl ? _step : 0;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Dokidoki'),
        bottom: PreferredSize(
          preferredSize: const Size.fromHeight(4),
          child:           LinearProgressIndicator(
            value: (step + 1) / 2,
            backgroundColor: Theme.of(context).colorScheme.surfaceContainerHighest,
          ),
        ),
      ),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  _StepIndicator(label: '1', active: step >= 0, title: '连接'),
                  const SizedBox(width: 8),
                  const Icon(Icons.chevron_right, size: 18),
                  const SizedBox(width: 8),
                  _StepIndicator(label: '2', active: step >= 1, title: '账号'),
                ],
              ),
              const SizedBox(height: 32),
              if (step == 0)
                SetupServerStep(onNext: () => setState(() => _step = 1))
              else
                const SetupAuthStep(),
            ],
          ),
        ),
      ),
    );
  }
}

class _StepIndicator extends StatelessWidget {
  const _StepIndicator({
    required this.label,
    required this.active,
    required this.title,
  });

  final String label;
  final bool active;
  final String title;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Row(
      children: [
        CircleAvatar(
          radius: 12,
          backgroundColor:
              active ? colorScheme.primary : colorScheme.surfaceContainerHighest,
          child: Text(
            label,
            style: TextStyle(
              fontSize: 12,
              color: active ? colorScheme.onPrimary : colorScheme.onSurfaceVariant,
            ),
          ),
        ),
        const SizedBox(width: 6),
        Text(title),
      ],
    );
  }
}
