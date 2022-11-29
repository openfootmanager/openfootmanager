# CONTRIBUTING

Thank your for taking the time to read this, and for showing your interest in supporting us!

There are several ways you can contribute with the project, whether you are a programmer or just a fan of this type of project, it means a lot if you can help us in any meaningful way. This game was born out of a dissatisfaction with alternatives in the market, and was built by a footbal fan, so I assume that if you're here, you want to be part of this and you're also a football fan.

If you can't code, but you have other skills that you can help us, don't worry, you can still do it. And if you can't do either of the things we proposed, you can still help us:

- Give us a star!
- Tweet about the project!
- Refer this project in your project's readme!
- Tell your friends about us!
- Share us on facebook!
- Donate to the project *(not available yet)*
- Make a video about it!
- Play it!

## How to contribute

If you really want to help us directly, thank you very much! We have a few jobs that you might be interested in:

- **Report a problem**  
  You can report bugs or issues you encounter in the game. Open an Issue and follow the steps to report the problem. Please read carefully the bug reporting issue template before submitting a new bug report. Provide as much information as you can to help us track the bug and solve it as fast as we possibly can.

- **Propose enhancements**  
  You can also propose new enhancements or improvements to the game. We're considering new ideas every day, and you can propose yours by opening an Issue and following the steps to propose enhancements. Just make sure to check the Issues page for similar ideas before opening up a new Issue. We don't want to flood the page with duplicated issues.

- **Documentation**  
  Do you think we can improve our documentation somehow? You can propose changes to the text, or write useful tutorials or examples on how to do certain things in the game.

- **Translation**  
  The game is still not translatable, but it soon will be. If you want to translate the game to your own language, you will be able to do that. We will soon provide a platform to do that. You will also be able to translate the documentation to your language.

- **Create new content**  
  You can create content to the game, like images, logos, database improvements, whatever you'd like. Soon this option will be available, and you will be able to submit your new content proposal easily.

## Submitting code

The most traditional way to contribute is to submit new code. **Openfoot Manager** is a GPLv3 licensed project, read the [LICENSE.md](LICENSE.md) before submitting your code.

Your code must be GPLv3 compliant, which means you understand that any code submitted here is original or also GPL-compliant, and must not depend on patents or copyrighted third-party content. Your code is subject to a free and open source license that will be available to the entire open source community.

Once you understand that concept, you're welcome to submit new code.

### Understanding the code

Soon I'll provide more explanations about the code. For now, read the code and try to understand it. See the tests to see what is expected from the code.

### Fork and Pull

We work with a [Fork & Pull](https://docs.github.com/en/github/collaborating-with-pull-requests/proposing-changes-to-your-work-with-pull-requests/about-pull-requests#fork--pull) method. Fork this repo, write your code in a feature branch (make sure it is up to date with the project's `develop` branch) and open a **Pull Request** to the `develop` repository, describing your changes or even referencing the **Issue** that inspired your code.

If you're working on a new feature that has no prior **Issue** related to it, please open an **Issue** describing the feature and then reference it in your new **Pull Request**.

### Code conventions

- Follow [PEP 8](https://www.python.org/dev/peps/pep-0008/). It's a must. Run autopep8 to ensure that your code is PEP 8-compliant.
- I'm still studying a way to make the entire code uniform, and I'm inclined to only accepting [Black](https://github.com/psf/black) formatted code.
- Make descriptive variable names, as best as you can.
- Use typehints as much as possible. Static typing helps others understand what you expect your code to do.
- Try to keep your code documented. If it's a complex method/function, use docstrings to explain what you need to do.
- Keep comments short and clean. Use them only when needed. Too many comments means that the code is not clean, and you should try to keep your code clean. You can use "TODO" comments to signal features that are not yet complete.
- Take advantage of Python's list and dict comprehensions.
- Tests are a must. More about it in the Tests section.

### Tests

Whenever you're adding a new feature, you must add tests to ensure that your feature works as expected. I started this project without tests, but then I reworked it entirely to include tests in as many features that I could think of. Tests help me keep my code clean and help me understand when something might not behave as I expected.

However, I'd prefer for you to use [pytest](https://pytest.org) instead of the Python's `unittest` library. Pytest is way more robust than `unittest`, and allows you to write short and effective tests. You should add your tests to the `ofm/tests` folder. Follow the conventions from the pytest community, and take a look at how the other tests are constructed to build your own tests.

I'll soon be adding CI/CD to this repo, and we can test pull requests on the fly. Your code should pass all the tests to be accepted. Make sure to run all tests before opening a Pull Request.

### Python versions

Python is rapidly changing, and I plan to adjust to Python's changes as we go. Currently supported Python version is 3.11.

This repo's CI/CD will use [tox](https://github.com/tox-dev/tox) to test version compatibility.
